# Qenlo Kotlin

Type-safe Kotlin/JVM bindings for Qenlo's embedded, durable vector database.

```kotlin
QenloCollection.memory(3).use { db ->
    db.add(Record(1u, 7u, 10, floatArrayOf(1f, 0f, 0f)))
    val response = db.search(floatArrayOf(1f, 0f, 0f), Filter(userId = 7u))
    check(response.results.first().id == 1uL)
}
```

The Maven release contains typed JVM code. Platform classifiers carry Qenlo's
native library for Windows, Linux, and macOS. Android uses the same C ABI through
the dedicated AAR packaging job.
