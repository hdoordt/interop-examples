#include <stdint.h> // uint32_t, uint8_t
#include <stddef.h> // size_t
#include <stdio.h>  // printf
#include <string.h> //strlen
#include "bindings/rust-in-c.h"
#include "bindings/Bsn.h"

void crc32_example()
{
    uint8_t data[] = {0, 1, 2, 3, 4, 5, 6};
    size_t data_length = 7;

    // Call extern "C" fn
    uint32_t hash = crc32(data, data_length);
    printf("Hash: %u\n", hash);
}

void bsn_cbindgen_example()
{
    char *bsn_strs[] = {"999996356", "1112223333"};
    for (int i = 0; i < 2; i++)
    {
        char *bsn_str = bsn_strs[i];
        // bsn_result is a tagged union
        BsnTryNewResult bsn_result = bsn_try_new(bsn_str);

        // check if result is ok
        if (bsn_result.tag == BsnTryNewResultOk)
        {
            printf("%s is a valid BSN!\n", bsn_str);
        }
        else
        {
            char buf[50];
            // To have functions 'return' strings, we use DiplomatWriteable
            error_display(&bsn_result.bsn_try_new_result_err, &buf, 50);

            printf("%s is not a valid BSN! Error: %s\n", bsn_str, buf);
        }
    }
}

/*
 * Call Diplomat-generated function
 */
void bsn_diplomat_example()
{
    char *bsn_strs[] = {"999996356", "1112223333"};
    for (int i = 0; i < 2; i++)
    {
        char *bsn_str = bsn_strs[i];
        // bsn_result is a tagged union
        diplomat_result_box_Bsn_BsnError bsn_result = Bsn_try_new_boxed(bsn_str, strlen(bsn_str));

        // check if result is ok
        if (bsn_result.is_ok)
        {
            printf("%s is a valid BSN!\n", bsn_str);
        }
        else
        {
            char buf[50];
            // To have functions 'return' strings, we use DiplomatWriteable
            DiplomatWriteable error_message_w = diplomat_simple_writeable(buf, 50);
            BsnError_fmt_display(&bsn_result.err, &error_message_w);

            printf("%s is not a valid BSN! Error: %s\n", bsn_str, error_message_w.buf);
        }
    }
}

int main()
{
    // Call extern "C" fn without params or return type
    say_hello();

    crc32_example();

    bsn_cbindgen_example();

    bsn_diplomat_example();

    return 0;
}
