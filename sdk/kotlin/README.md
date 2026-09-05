# Qenlo Kotlin SDK

Type-safe Kotlin/JVM and Android bindings for **Qenlo** — the embedded, durable vector database written in Rust.

Qenlo provides exact filtered cosine vector search with atomic transactions, write-ahead logging (WAL), deterministic ordering, and Kotlin unsigned integer support.

## Installation

### Gradle (Kotlin DSL)

```kotlin
repositories {
    mavenCentral()
}

dependencies {
    implementation("dev.qenlo:qenlo:0.1.0")
}
```

### Maven (`pom.xml`)

```xml
<dependency>
    <groupId>dev.qenlo</groupId>
    <artifactId>qenlo</artifactId>
    <version>0.1.0</version>
</dependency>
```

Native libraries are embedded in the JAR and automatically extracted via JNA on `linux-x64`, `windows-x64`, and `darwin-arm64`.

Collections use exhaustive CPU search by default. A desktop artifact built with portable GPU support can opt into automatic routing or require the GPU:

```kotlin
val options = CollectionOptions(
    backend = ExecutionMode.AUTOMATIC,
    gpuFilterMode = GpuFilterMode.GPU_PREDICATE,
    gpuAllocationBudgetBytes = 512uL * 1024uL * 1024uL,
)
val db = QenloCollection.memory(384, options)
```

Automatic mode exposes the actual route and fallback in `ExecutionReport`. Required-GPU mode fails during construction when the native artifact or host cannot provide the backend.

---

## Quickstart

### In-Memory Collection with `AutoCloseable` (`use`)

```kotlin
import dev.qenlo.*

fun main() {
    // Automatically freed upon block completion via .use { }
    QenloCollection.memory(dimension = 3).use { db ->
        // Insert records with unsigned IDs and user IDs
        db.add(Record(
            id = 1uL,
            userId = 42uL,
            timestamp = 100L,
            vector = floatArrayOf(1.0f, 0.0f, 0.0f)
        ))

        db.add(Record(
            id = 2uL,
            userId = 42uL,
            timestamp = 200L,
            vector = floatArrayOf(0.0f, 1.0f, 0.0f)
        ))

        // Query with combined metadata filters
        val response = db.search(
            query = floatArrayOf(1.0f, 0.0f, 0.0f),
            filter = Filter(
                userId = 42uL,
                timestampLower = 50L,
                timestampUpper = 150L
            ),
            k = 5
        )

        for (hit in response.results) {
            println("Matched ID: ${hit.id}, Distance: ${hit.distance}")
        }

        println("Executed on: ${response.report.actualBackend} in ${response.report.totalDurationNs}ns")
    }
}
```

---

## Durable Storage Across Restarts

```kotlin
import dev.qenlo.*

val path = "./vectors.qenlo"

// 1. Create a persistent collection
QenloCollection.create(path, dimension = 128).use { db ->
    db.add(myRecord)
    db.flush()
}

// 2. Open after restart
QenloCollection.open(path, dimension = 128).use { db ->
    val response = db.search(myQuery, Filter(userId = 7uL))
    println("Results: ${response.results.size}")
}
```

---

## Portable `.qn` Interchange Files

```kotlin
// Export current collection to .qn snapshot
db.exportQn("snapshot.qn")

// Import snapshot into a new in-memory collection
QenloCollection.importQn("snapshot.qn", dimension = 128).use { snapshotDb ->
    println("Live rows: ${snapshotDb.stats().liveRows}")
}
```

---

## Batch Operations

```kotlin
val batch = listOf(
    Record(10uL, 1uL, 1000L, floatArrayOf(0.1f, 0.2f, 0.3f)),
    Record(11uL, 1uL, 1001L, floatArrayOf(0.4f, 0.5f, 0.6f)),
)

// Atomic batch insert
db.addBatch(batch)

// Batch delete by ID
db.deleteBatch(listOf(10uL, 11uL))
```

---

## License

Dual-licensed under **MIT** or **Apache-2.0** at your option.
