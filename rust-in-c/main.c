#include <stdint.h> // uint32_t, uint8_t
#include <stddef.h> // size_t
#include <stdio.h> // printf
#include "rust-in-c.h"

int main() { 
    say_hello();

    uint8_t data[] = { 0,1,2,3,4,5,6 };
    size_t data_length = 7;

    uint32_t hash = crc32(data, data_length);

    printf("Hash: %u\n", hash);

    return 0;
}
