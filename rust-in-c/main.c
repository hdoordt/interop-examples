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

    uint32_t hash = crc32(data, data_length);
    printf("Hash: %u\n", hash);
}

void bsn_cbindgen_example()
{
    const char *bsn_strs[] = {"999996356", "1112223333", "bogus!", (char[]){0xFE, 0xFF, '\0'}};
    for (int i = 0; i < 4; i++)
    {
        const char *bsn_str = bsn_strs[i];
        BsnTryNewResult bsn_result = bsn_try_new(bsn_str);
        if (bsn_result.tag == BsnTryNewResultOk)
        {
            printf("%s is a valid BSN!\n", bsn_str);
        }
        else
        {
            // Make sure the buffer is big enough
            char buf[50];
            error_display(&bsn_result.bsn_try_new_result_err, buf, 50);
            printf("%s is not a valid BSN! Error: %s\n", bsn_str, buf);
        }
    }
}

void bsn_diplomat_example()
{
    const char *bsn_strs[] = {"999996356", "1112223333", "bogus!", (char[]){0xFE, 0xFF, '\0'}};
    for (int i = 0; i < 4; i++)
    {
        const char *bsn_str = bsn_strs[i];
        diplomat_result_box_Bsn_BsnError bsn_result = Bsn_try_new_boxed(bsn_str, strlen(bsn_str));

        if (bsn_result.is_ok)
        {
            printf("%s is a valid BSN!\n", bsn_str);
        }
        else
        {
            char buf[50];
            DiplomatWriteable error_message_w = diplomat_simple_writeable(buf, 50);
            BsnError_fmt_display(&bsn_result.err, &error_message_w);

            printf("%s is not a valid BSN! Error: %s\n", bsn_str, error_message_w.buf);
        }
    }
}

int main()
{
    say_hello();
    crc32_example();
    bsn_cbindgen_example();
    bsn_diplomat_example();


    return 0;
}
