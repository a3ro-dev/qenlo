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
typedef struct QenloSnapshot QenloSnapshot;
typedef struct QenloSearchResults QenloSearchResults;

enum {
    QENLO_BACKEND_CPU = 0,
    QENLO_BACKEND_AUTOMATIC = 1,
    QENLO_BACKEND_GPU_REQUIRED = 2,
};

enum {
    QENLO_GPU_FILTER_CPU_MASK = 0,
    QENLO_GPU_FILTER_CPU_ROWS = 1,
    QENLO_GPU_FILTER_PREDICATE = 2,
};

QENLO_API QenloCollection *qenlo_collection_new(size_t dimension);
QENLO_API QenloCollection *qenlo_collection_new_configured(
    size_t dimension, uint32_t backend, uint32_t gpu_filter_mode,
    uint64_t gpu_allocation_budget_bytes);
QENLO_API QenloCollection *qenlo_collection_create(const char *path, size_t dimension);
QENLO_API QenloCollection *qenlo_collection_create_configured(
    const char *path, size_t dimension, uint32_t backend,
    uint32_t gpu_filter_mode, uint64_t gpu_allocation_budget_bytes);
QENLO_API QenloCollection *qenlo_collection_open(const char *path, size_t dimension);
QENLO_API QenloCollection *qenlo_collection_open_configured(
    const char *path, size_t dimension, uint32_t backend,
    uint32_t gpu_filter_mode, uint64_t gpu_allocation_budget_bytes);
QENLO_API QenloCollection *qenlo_collection_import_qn(const char *path, size_t dimension);
QENLO_API QenloCollection *qenlo_collection_import_qn_configured(
    const char *path, size_t dimension, uint32_t backend,
    uint32_t gpu_filter_mode, uint64_t gpu_allocation_budget_bytes);
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
QENLO_API QenloSearchResults *qenlo_search_results_new(
    QenloCollection *collection, const float *query, size_t query_len,
    bool has_user_id, uint64_t user_id, bool has_lower, int64_t lower,
    bool has_upper, int64_t upper, size_t k);
QENLO_API int32_t qenlo_search_results_len(QenloSearchResults *results,
                                           size_t *rows);
QENLO_API int32_t qenlo_search_results_copy(QenloSearchResults *results,
                                            uint64_t *ids, size_t ids_len,
                                            float *distances,
                                            size_t distances_len);
QENLO_API char *qenlo_search_results_report_json(QenloSearchResults *results);
QENLO_API void qenlo_search_results_free(QenloSearchResults *results);
QENLO_API QenloSnapshot *qenlo_snapshot_new(QenloCollection *collection,
                                            bool has_user_id, uint64_t user_id,
                                            bool has_lower, int64_t lower,
                                            bool has_upper, int64_t upper);
QENLO_API int32_t qenlo_snapshot_info(QenloSnapshot *snapshot, uint64_t *generation,
                                      size_t *rows, size_t *dimension);
QENLO_API int32_t qenlo_snapshot_copy(QenloSnapshot *snapshot, uint64_t *ids,
                                      size_t ids_len, float *vectors,
                                      size_t vectors_len);
QENLO_API void qenlo_snapshot_free(QenloSnapshot *snapshot);
QENLO_API int32_t qenlo_collection_generation(QenloCollection *collection,
                                              uint64_t *generation);
QENLO_API char *qenlo_stats(QenloCollection *collection);
QENLO_API int32_t qenlo_export_qn(QenloCollection *collection, const char *path);
QENLO_API int32_t qenlo_flush(QenloCollection *collection);
QENLO_API int32_t qenlo_close(QenloCollection *collection);
QENLO_API void qenlo_collection_free(QenloCollection *collection);
QENLO_API char *qenlo_last_error(void);
QENLO_API void qenlo_string_free(char *value);

#ifdef __cplusplus
}
#endif

#endif
