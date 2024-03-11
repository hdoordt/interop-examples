#include "cxx-pretty-json/src/main.rs.h"
#include <iostream>
#include <iterator>
#include <string>
#include <vector>

int main() {
  // Read json from stdin.
  std::istreambuf_iterator<char> begin{std::cin}, end;
  std::vector<unsigned char> input{begin, end};
  rust::Slice<const uint8_t> slice{input.data(), input.size()};

  // Prettify using serde_json and serde_transcode.
  std::string output;
  prettify_json(slice, output);

  // Write to stdout.
  std::cout << output << std::endl;
}
