package dev.qenlo

import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.NativeLibrary
import com.sun.jna.Pointer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.nio.file.Path
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.util.concurrent.locks.ReentrantReadWriteLock
import kotlin.concurrent.read
import kotlin.concurrent.write

/** A validation, lifecycle, storage, or native execution failure. */
public class QenloException(message: String) : RuntimeException(message)

/** Requested execution policy for collection searches. */
public enum class ExecutionMode(internal val nativeValue: Int) {
    CPU(0),
    AUTOMATIC(1),
    GPU_REQUIRED(2),
}

/** Eligibility preparation strategy used by the portable GPU backend. */
public enum class GpuFilterMode(internal val nativeValue: Int) {
    CPU_MASK(0),
    CPU_ELIGIBLE_ROWS(1),
    GPU_PREDICATE(2),
}

/** Native execution and allocation settings for a collection. */
public data class CollectionOptions(
    public val backend: ExecutionMode = ExecutionMode.CPU,
    public val gpuFilterMode: GpuFilterMode = GpuFilterMode.GPU_PREDICATE,
    public val gpuAllocationBudgetBytes: ULong = 512uL * 1024uL * 1024uL,
) {
    init { require(gpuAllocationBudgetBytes > 0uL) { "gpu allocation budget must be positive" } }
}

/** One canonical vector and its filterable metadata. */
public data class Record(
    public val id: ULong,
    public val userId: ULong,
    public val timestamp: Long,
    public val vector: FloatArray,
) {
    override fun equals(other: Any?): Boolean = other is Record && id == other.id && userId == other.userId && timestamp == other.timestamp && vector.contentEquals(other.vector)
    override fun hashCode(): Int = 31 * (31 * (31 * id.hashCode() + userId.hashCode()) + timestamp.hashCode()) + vector.contentHashCode()
}

/** Optional user equality and lower-inclusive, upper-exclusive timestamp bounds. */
public data class Filter(
    public val userId: ULong? = null,
    public val timestampLower: Long? = null,
    public val timestampUpper: Long? = null,
)

public data class SearchResult(public val id: ULong, public val distance: Float)

public data class ExecutionReport(
    public val operationId: ULong,
    public val requestedBackend: String,
    public val actualBackend: String,
    public val algorithm: String,
    public val filterExecution: String,
    public val indexGeneration: ULong,
    public val rebuilt: Boolean,
    public val routingReason: String?,
    public val fallbackReason: String?,
    public val totalDurationNs: ULong,
    public val lockWaitNs: ULong,
    public val eligibleRows: ULong?,
    public val uploadBytes: ULong?,
    public val readbackBytes: ULong?,
    public val allocationBytes: ULong?,
    public val dispatchCount: UInt?,
    public val candidates: ULong?,
    public val batchSize: Int,
)

public data class SearchResponse(public val results: List<SearchResult>, public val report: ExecutionReport)

public data class CollectionStats(
    public val dimension: Int,
    public val rows: Int,
    public val liveRows: Int,
    public val generation: ULong,
    public val preparedGeneration: ULong?,
    public val durableGeneration: ULong?,
    public val recoveredInterruptedWrite: Boolean,
    public val closed: Boolean,
)

private interface QenloNative : Library {
    fun qenlo_collection_new(dimension: Long): Pointer?
    fun qenlo_collection_new_configured(dimension: Long, backend: Int, gpuFilterMode: Int, gpuAllocationBudgetBytes: Long): Pointer?
    fun qenlo_collection_create(path: String, dimension: Long): Pointer?
    fun qenlo_collection_create_configured(path: String, dimension: Long, backend: Int, gpuFilterMode: Int, gpuAllocationBudgetBytes: Long): Pointer?
    fun qenlo_collection_open(path: String, dimension: Long): Pointer?
    fun qenlo_collection_open_configured(path: String, dimension: Long, backend: Int, gpuFilterMode: Int, gpuAllocationBudgetBytes: Long): Pointer?
    fun qenlo_collection_import_qn(path: String, dimension: Long): Pointer?
    fun qenlo_collection_import_qn_configured(path: String, dimension: Long, backend: Int, gpuFilterMode: Int, gpuAllocationBudgetBytes: Long): Pointer?
    fun qenlo_add(handle: Pointer, id: Long, userId: Long, timestamp: Long, vector: FloatArray, vectorLen: Long): Int
    fun qenlo_add_batch(handle: Pointer, ids: LongArray, userIds: LongArray, timestamps: LongArray, vectors: FloatArray, rows: Long, dimension: Long): Int
    fun qenlo_delete(handle: Pointer, id: Long): Int
    fun qenlo_delete_batch(handle: Pointer, ids: LongArray, rows: Long): Int
    fun qenlo_search(handle: Pointer, query: FloatArray, queryLen: Long, hasUserId: Byte, userId: Long, hasLower: Byte, lower: Long, hasUpper: Byte, upper: Long, k: Long): Pointer?
    fun qenlo_stats(handle: Pointer): Pointer?
    fun qenlo_export_qn(handle: Pointer, path: String): Int
    fun qenlo_flush(handle: Pointer): Int
    fun qenlo_close(handle: Pointer): Int
    fun qenlo_collection_free(handle: Pointer)
    fun qenlo_last_error(): Pointer?
    fun qenlo_string_free(value: Pointer)
}

private object NativeApi {
    val value: QenloNative by lazy {
        val explicit = System.getenv("QENLO_LIBRARY_PATH")?.let { Path.of(it) }
        val library = when {
            explicit?.toFile()?.isFile == true -> explicit.toAbsolutePath().toString()
            explicit != null -> {
                NativeLibrary.addSearchPath("qenlo_ffi", explicit.toAbsolutePath().toString())
                "qenlo_ffi"
            }
            else -> bundledLibrary()?.toString() ?: "qenlo_ffi"
        }
        Native.load(library, QenloNative::class.java)
    }

    private fun bundledLibrary(): Path? {
        val os = System.getProperty("os.name").lowercase()
        val arch = System.getProperty("os.arch").lowercase()
        val platform = when {
            os.contains("win") && arch in setOf("amd64", "x86_64") -> "windows-x64"
            os.contains("linux") && arch in setOf("amd64", "x86_64") -> "linux-x64"
            os.contains("mac") && arch in setOf("aarch64", "arm64") -> "darwin-arm64"
            else -> return null
        }
        val file = when {
            os.contains("win") -> "qenlo_ffi.dll"
            os.contains("mac") -> "libqenlo_ffi.dylib"
            else -> "libqenlo_ffi.so"
        }
        val resource = "/qenlo/$platform/$file"
        val input = NativeApi::class.java.getResourceAsStream(resource) ?: return null
        val directory = Files.createTempDirectory("qenlo-native-").also { it.toFile().deleteOnExit() }
        val target = directory.resolve(file)
        input.use { Files.copy(it, target, StandardCopyOption.REPLACE_EXISTING) }
        target.toFile().deleteOnExit()
        return target
    }
}

@Serializable
private data class WireHit(val id: String, val distance: Float)

@Serializable
private data class WireReport(
    @SerialName("operation_id") val operationId: String,
    @SerialName("requested_backend") val requestedBackend: String,
    @SerialName("actual_backend") val actualBackend: String,
    val algorithm: String,
    @SerialName("filter_execution") val filterExecution: String,
    @SerialName("index_generation") val indexGeneration: String,
    val rebuilt: Boolean,
    @SerialName("routing_reason") val routingReason: String?,
    @SerialName("fallback_reason") val fallbackReason: String?,
    @SerialName("total_duration_ns") val totalDurationNs: String,
    @SerialName("lock_wait_ns") val lockWaitNs: String,
    @SerialName("eligible_rows") val eligibleRows: String?,
    @SerialName("upload_bytes") val uploadBytes: String?,
    @SerialName("readback_bytes") val readbackBytes: String?,
    @SerialName("allocation_bytes") val allocationBytes: String?,
    @SerialName("dispatch_count") val dispatchCount: UInt?,
    val candidates: String?,
    @SerialName("batch_size") val batchSize: Int,
)

@Serializable
private data class WireSearch(val results: List<WireHit>, val report: WireReport)

@Serializable
private data class WireStats(
    val dimension: Int,
    val rows: Int,
    @SerialName("live_rows") val liveRows: Int,
    val generation: String,
    @SerialName("prepared_generation") val preparedGeneration: String?,
    @SerialName("durable_generation") val durableGeneration: String?,
    @SerialName("recovered_interrupted_write") val recoveredInterruptedWrite: Boolean,
    val closed: Boolean,
)

/** An owned Qenlo collection. Close explicitly or use [use]. */
public class QenloCollection private constructor(handle: Pointer?, public val dimension: Int) : AutoCloseable {
    private val lock: ReentrantReadWriteLock = ReentrantReadWriteLock()
    private var handle: Pointer? = handle ?: throw QenloException(lastError())

    public companion object {
        public fun memory(dimension: Int, options: CollectionOptions = CollectionOptions()): QenloCollection {
            require(dimension > 0) { "dimension must be positive" }
            return QenloCollection(
                NativeApi.value.qenlo_collection_new_configured(
                    dimension.toLong(), options.backend.nativeValue, options.gpuFilterMode.nativeValue,
                    options.gpuAllocationBudgetBytes.toLong(),
                ),
                dimension,
            )
        }

        public fun create(path: Path, dimension: Int, options: CollectionOptions = CollectionOptions()): QenloCollection {
            require(dimension > 0) { "dimension must be positive" }
            return QenloCollection(
                NativeApi.value.qenlo_collection_create_configured(
                    path.toString(), dimension.toLong(), options.backend.nativeValue,
                    options.gpuFilterMode.nativeValue, options.gpuAllocationBudgetBytes.toLong(),
                ),
                dimension,
            )
        }

        public fun open(path: Path, dimension: Int, options: CollectionOptions = CollectionOptions()): QenloCollection {
            require(dimension > 0) { "dimension must be positive" }
            return QenloCollection(
                NativeApi.value.qenlo_collection_open_configured(
                    path.toString(), dimension.toLong(), options.backend.nativeValue,
                    options.gpuFilterMode.nativeValue, options.gpuAllocationBudgetBytes.toLong(),
                ),
                dimension,
            )
        }

        public fun importQn(path: Path, dimension: Int, options: CollectionOptions = CollectionOptions()): QenloCollection {
            require(dimension > 0) { "dimension must be positive" }
            return QenloCollection(
                NativeApi.value.qenlo_collection_import_qn_configured(
                    path.toString(), dimension.toLong(), options.backend.nativeValue,
                    options.gpuFilterMode.nativeValue, options.gpuAllocationBudgetBytes.toLong(),
                ),
                dimension,
            )
        }

        private fun lastError(): String = takeString(NativeApi.value.qenlo_last_error(), false)

        private fun takeString(pointer: Pointer?, failOnNull: Boolean = true): String {
            if (pointer == null) {
                if (failOnNull) throw QenloException(lastError())
                return "unknown Qenlo native error"
            }
            return try { pointer.getString(0, Charsets.UTF_8.name()) } finally { NativeApi.value.qenlo_string_free(pointer) }
        }
    }

    private fun openHandle(): Pointer = handle ?: throw QenloException("collection is closed")
    private fun check(status: Int) { if (status != 0) throw QenloException(lastError()) }
    private fun vector(value: FloatArray): FloatArray {
        require(value.size == dimension) { "expected vector dimension $dimension, got ${value.size}" }
        return value
    }

    public fun add(record: Record): Unit = lock.read {
        check(NativeApi.value.qenlo_add(openHandle(), record.id.toLong(), record.userId.toLong(), record.timestamp, vector(record.vector), dimension.toLong()))
    }

    public fun addBatch(records: List<Record>): Unit = lock.read {
        if (records.isEmpty()) return@read
        val ids = LongArray(records.size)
        val users = LongArray(records.size)
        val timestamps = LongArray(records.size)
        val vectors = FloatArray(records.size * dimension)
        records.forEachIndexed { row, record ->
            ids[row] = record.id.toLong(); users[row] = record.userId.toLong(); timestamps[row] = record.timestamp
            vector(record.vector).copyInto(vectors, row * dimension)
        }
        check(NativeApi.value.qenlo_add_batch(openHandle(), ids, users, timestamps, vectors, records.size.toLong(), dimension.toLong()))
    }

    public fun delete(id: ULong): Unit = lock.read { check(NativeApi.value.qenlo_delete(openHandle(), id.toLong())) }
    public fun deleteBatch(ids: List<ULong>): Unit = lock.read {
        if (ids.isNotEmpty()) check(NativeApi.value.qenlo_delete_batch(openHandle(), ids.map(ULong::toLong).toLongArray(), ids.size.toLong()))
    }

    public fun search(query: FloatArray, filter: Filter = Filter(), k: Int = 10): SearchResponse = lock.read {
        require(k in 1..64) { "k must be in 1..=64" }
        val pointer = NativeApi.value.qenlo_search(openHandle(), vector(query), dimension.toLong(), (if (filter.userId != null) 1 else 0).toByte(), filter.userId?.toLong() ?: 0, (if (filter.timestampLower != null) 1 else 0).toByte(), filter.timestampLower ?: 0, (if (filter.timestampUpper != null) 1 else 0).toByte(), filter.timestampUpper ?: 0, k.toLong())
        val wire = Json.decodeFromString<WireSearch>(takeString(pointer))
        SearchResponse(
            wire.results.map { SearchResult(it.id.toULong(), it.distance) },
            wire.report.run { ExecutionReport(operationId.toULong(), requestedBackend, actualBackend, algorithm, filterExecution, indexGeneration.toULong(), rebuilt, routingReason, fallbackReason, totalDurationNs.toULong(), lockWaitNs.toULong(), eligibleRows?.toULong(), uploadBytes?.toULong(), readbackBytes?.toULong(), allocationBytes?.toULong(), dispatchCount, candidates?.toULong(), batchSize) },
        )
    }

    public fun stats(): CollectionStats = lock.read {
        val wire = Json.decodeFromString<WireStats>(takeString(NativeApi.value.qenlo_stats(openHandle())))
        CollectionStats(wire.dimension, wire.rows, wire.liveRows, wire.generation.toULong(), wire.preparedGeneration?.toULong(), wire.durableGeneration?.toULong(), wire.recoveredInterruptedWrite, wire.closed)
    }

    public fun flush(): Unit = lock.read { check(NativeApi.value.qenlo_flush(openHandle())) }

    public fun exportQn(path: Path): Unit = lock.read {
        check(NativeApi.value.qenlo_export_qn(openHandle(), path.toString()))
    }

    override fun close(): Unit = lock.write {
        handle?.let { value ->
            handle = null
            val status = NativeApi.value.qenlo_close(value)
            NativeApi.value.qenlo_collection_free(value)
            check(status)
        }
    }
}
