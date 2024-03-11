#include "cxx-crc32fast/include/crc32fast.h"
#include "cxx-crc32fast/src/main.rs.h"
#include <iostream>
#include <iomanip>

int main() {
    // Read input from stdin 
    std::istreambuf_iterator<char> begin{std::cin}, end;
    std::vector<unsigned char> input{begin, end};
    rust::Slice<const uint8_t> slice{input.data(), input.size()}; // drop the linefeed
  
    Hasher &h = *init();
    h.update(slice);
    uint32_t output = h.finalize();

    // Write to stdout.
    std::cout << std::setw(8) << std::setfill('0') << std::hex << output << std::endl;
}
