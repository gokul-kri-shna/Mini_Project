#include <stdio.h>
#include <stdlib.h>

// Idiom 3: Ownership Created Through T** Out-Parameter
// Callee allocates new heap memory and transfers ownership back to caller via out-parameter.
void create_buffer(int **out_ptr, int size) {
    *out_ptr = (int *)malloc(size * sizeof(int));
    for (int i = 0; i < size; i++) {
        (*out_ptr)[i] = (i + 1) * 10;
    }
}

int main() {
    int *my_array = NULL;
    create_buffer(&my_array, 5);
    printf("Array created: %d, %d, %d\n", my_array[0], my_array[1], my_array[2]);
    free(my_array);
    return 0;
}
