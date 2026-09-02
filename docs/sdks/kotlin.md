# Kotlin & Android SDK

JNI-backed bindings for JVM and Android applications.

## Gradle Setup

```kotlin
dependencies {
    implementation("org.gobitsnbytes:qenlo:0.1.0-alpha.1")
}
```

## Quick Example

```kotlin
import org.gobitsnbytes.qenlo.QenloCollection
import org.gobitsnbytes.qenlo.Record
import org.gobitsnbytes.qenlo.Filter

QenloCollection.memory(dim = 3).use { db ->
    db.add(Record(id = 1u, userId = 7u, timestamp = 1700000000L, vector = floatArrayOf(1f, 0f, 0f)))
    val response = db.search(floatArrayOf(1f, 0f, 0f), Filter(userId = 7u), topK = 5)
    println("Matches: ${response.results.size}")
}
```
