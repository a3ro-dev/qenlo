#ifndef QENLO_H
#define QENLO_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef _WIN32
#define QENLO_API __declspec(dllexport)
#else
#define QENLO_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct QenloCollection QenloCollection;

QENLO_API QenloCollection *qenlo_collection_new(size_t dimension);
QENLO_API QenloCollection *qenlo_collection_create(const char *path, size_t dimension);
QENLO_API QenloCollection *qenlo_collection_open(const char *path, size_t dimension);
QENLO_API int32_t qenlo_add(QenloCollection *collection, uint64_t id, uint64_t user_id,
                            int64_t timestamp, const float *vector, size_t vector_len);
QENLO_API int32_t qenlo_add_batch(QenloCollection *collection, const uint64_t *ids,
                                  const uint64_t *user_ids, const int64_t *timestamps,
                                  const float *vectors, size_t rows, size_t dimension);
QENLO_API int32_t qenlo_delete(QenloCollection *collection, uint64_t id);
QENLO_API int32_t qenlo_delete_batch(QenloCollection *collection, const uint64_t *ids,
                                     size_t rows);
QENLO_API char *qenlo_search(QenloCollection *collection, const float *query,
                             size_t query_len, bool has_user_id, uint64_t user_id,
                             bool has_lower, int64_t lower, bool has_upper,
                             int64_t upper, size_t k);
QENLO_API char *qenlo_stats(QenloCollection *collection);
QENLO_API int32_t qenlo_flush(QenloCollection *collection);
QENLO_API int32_t qenlo_close(QenloCollection *collection);
QENLO_API void qenlo_collection_free(QenloCollection *collection);
QENLO_API char *qenlo_last_error(void);
QENLO_API void qenlo_string_free(char *value);

#ifdef __cplusplus
}
#endif

#endif
