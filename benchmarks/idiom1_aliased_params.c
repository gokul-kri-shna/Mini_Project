#include <stdio.h>

// Idiom 1: Aliased Mutable Parameters
// Parameter 'a' and 'b' look independent in function body,
// but alias at specific call sites (e.g., alias_func(&val, &val)).
void modify_pair(int *a, int *b) {
    *a = *a + 10;
    *b = *b * 2;
}

int main() {
    int val = 5;
    printf("Before: %d\n", val);
    // Call site where arguments alias:
    modify_pair(&val, &val);
    printf("After: %d\n", val);
    return 0;
}
