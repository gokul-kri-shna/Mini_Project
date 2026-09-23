#include <stdio.h>

// Idiom 5: Union's Active Variant Carried Across Call Boundary
// Union active field is written in one function and read in another function.
union Value {
    int i;
    float f;
};

void set_integer_variant(union Value *v, int val) {
    v->i = val; // Active variant set to integer
}

void print_integer_variant(union Value *v) {
    printf("Active integer variant: %d\n", v->i); // Read across call boundary
}

int main() {
    union Value val;
    set_integer_variant(&val, 100);
    print_integer_variant(&val);
    return 0;
}
