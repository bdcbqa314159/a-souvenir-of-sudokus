// WebAssembly entry point: the whole engine behind one C function.
// JS side (via cwrap): souvenir_cmd(requestJson) -> responseJson.
// See souvenir/api.hpp for the command set.
#include <emscripten/emscripten.h>

#include <string>

#include <souvenir/api.hpp>

extern "C" {

EMSCRIPTEN_KEEPALIVE const char *souvenir_cmd(const char *request) {
  // ponytail: static buffer — wasm build is single-threaded, and this saves the
  // caller a malloc/free dance; revisit if pthreads ever get enabled
  static std::string response;
  response = souvenir::apply_command(request ? request : "");
  return response.c_str();
}

} // extern "C"
