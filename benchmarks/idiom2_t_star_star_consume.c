#include <stdio.h>
#include <stdlib.h>

// Idiom 2: Ownership Consumed Through T** Argument
// Callee frees the pointee through double indirection pointer and sets to NULL.
void release_buffer(int **ptr) {
    if (ptr && *ptr) {
        free(*ptr);
        *ptr = NULL;
    }
}

int main() {
    int *data = (int *)malloc(10 * sizeof(int));
    data[0] = 42;
    printf("Data before release: %d\n", data[0]);
    release_buffer(&data);
    if (data == NULL) {
        printf("Buffer successfully released!\n");
    }
    return 0;
}
