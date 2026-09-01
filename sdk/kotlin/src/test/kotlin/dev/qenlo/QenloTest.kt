package dev.qenlo

import org.junit.jupiter.api.io.TempDir
import java.nio.file.Path
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class QenloTest {
    private fun fixture() = listOf(
        Record(9u, 7u, -5, floatArrayOf(1f, 0f, 0f)),
        Record(2u, 7u, 0, floatArrayOf(2f, 0f, 0f)),
        Record(4u, 8u, 10, floatArrayOf(0f, 1f, 0f)),
        Record(6u, 7u, 20, floatArrayOf(0f, 0f, 1f)),
    )

    @Test fun `typed filters ordering and telemetry`() {
        QenloCollection.memory(3).use { db ->
            db.addBatch(fixture())
            val response = db.search(floatArrayOf(1f, 0f, 0f), Filter(7u, -5, 20))
            assertEquals(listOf(2uL, 9uL), response.results.map(SearchResult::id))
            assertEquals("Cpu", response.report.actualBackend)
            assertEquals("Exact", response.report.algorithm)
            assertTrue(response.report.operationId > 0u)
        }
    }

    @Test fun `atomic batches and non reusable ids`() {
        QenloCollection.memory(3).use { db ->
            db.add(fixture()[0])
            assertFailsWith<QenloException> { db.addBatch(listOf(fixture()[1], fixture()[0])) }
            assertEquals(1, db.stats().rows)
            db.delete(9u)
            assertFailsWith<QenloException> { db.add(fixture()[0]) }
        }
    }

    @Test fun `durable reopen`(@TempDir root: Path) {
        val path = root.resolve("vectors.qenlo")
        QenloCollection.create(path, 3).use { db -> db.addBatch(fixture()); db.deleteBatch(listOf(2u, 4u)); db.flush() }
        QenloCollection.open(path, 3).use { db -> assertEquals(2, db.stats().liveRows) }
    }

    @Test fun `portable qn round trip`(@TempDir root: Path) {
        val path = root.resolve("vectors.qn")
        QenloCollection.memory(3).use { db ->
            db.addBatch(fixture()); db.delete(9u); db.exportQn(path)
            assertFailsWith<QenloException> { db.exportQn(path) }
        }
        QenloCollection.importQn(path, 3).use { db ->
            assertEquals(5uL, db.stats().generation)
            assertEquals(3, db.stats().liveRows)
        }
    }

    @Test fun `validation and lifecycle`() {
        val db = QenloCollection.memory(3)
        assertFailsWith<IllegalArgumentException> { db.add(Record(1u, 1u, 0, floatArrayOf(1f))) }
        assertFailsWith<IllegalArgumentException> { db.search(floatArrayOf(1f, 0f, 0f), k = 0) }
        assertFailsWith<QenloException> { db.add(Record(1u, 1u, 0, floatArrayOf(0f, 0f, 0f))) }
        db.close(); db.close()
        assertFailsWith<QenloException> { db.stats() }
    }
}
