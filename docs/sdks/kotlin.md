# Kotlin/JVM SDK (preview)

JNA bindings for JVM applications. Android packaging and bridge validation are tracked separately, so this JVM artifact does not provide Android support on its own.

Alpha.5 ships a preview JAR in the GitHub release. Maven Central publication is
not guaranteed; build from `sdk/kotlin` with `gradle build` when no registry
artifact is available.

## Quick example

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

The default execution mode is `ExecutionMode.CPU`. Desktop native binaries with portable GPU support also accept `CollectionOptions` configured with `AUTOMATIC` or `GPU_REQUIRED`, a `GpuFilterMode`, and a byte allocation budget. Automatic mode records the chosen route and fallback in `ExecutionReport`. GPU-required mode fails if the host environment cannot provide GPU execution.
