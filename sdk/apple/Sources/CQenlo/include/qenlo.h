#ifndef QENLO_H
#define QENLO_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
typedef struct QenloCollection QenloCollection;
QenloCollection *qenlo_collection_new(size_t dimension);
QenloCollection *qenlo_collection_create(const char *path, size_t dimension);
QenloCollection *qenlo_collection_open(const char *path, size_t dimension);
int32_t qenlo_add(QenloCollection *, uint64_t, uint64_t, int64_t, const float *, size_t);
int32_t qenlo_add_batch(QenloCollection *, const uint64_t *, const uint64_t *, const int64_t *, const float *, size_t, size_t);
int32_t qenlo_delete(QenloCollection *, uint64_t);
int32_t qenlo_delete_batch(QenloCollection *, const uint64_t *, size_t);
char *qenlo_search(QenloCollection *, const float *, size_t, bool, uint64_t, bool, int64_t, bool, int64_t, size_t);
char *qenlo_stats(QenloCollection *);
int32_t qenlo_flush(QenloCollection *);
int32_t qenlo_close(QenloCollection *);
void qenlo_collection_free(QenloCollection *);
char *qenlo_last_error(void);
void qenlo_string_free(char *);
#endif
