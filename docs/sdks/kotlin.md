# Kotlin/JVM SDK

JNA bindings for JVM applications. Android packaging and bridge validation are
tracked separately; a JVM artifact alone is not Android support.

## Gradle Setup

```kotlin
dependencies {
    implementation("dev.qenlo:qenlo:0.1.0-alpha.2")
}
```

## Quick Example

```kotlin
import dev.qenlo.QenloCollection
import dev.qenlo.Record
import dev.qenlo.Filter

QenloCollection.memory(dimension = 3).use { db ->
    db.add(Record(id = 1u, userId = 7u, timestamp = 1700000000L, vector = floatArrayOf(1f, 0f, 0f)))
    val response = db.search(floatArrayOf(1f, 0f, 0f), Filter(userId = 7u), k = 5)
    println("Matches: ${response.results.size}")
}
```

The default execution mode is `ExecutionMode.CPU`. Desktop native artifacts with
portable GPU support also accept `CollectionOptions` with `AUTOMATIC` or
`GPU_REQUIRED`, a `GpuFilterMode`, and a byte allocation budget. Automatic mode
records the actual route and fallback in `ExecutionReport`; required mode fails
if the artifact or host cannot provide the backend.
