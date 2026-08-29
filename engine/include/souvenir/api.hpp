// a-souvenir-of-sudokus — JSON command surface.
// One stateless function: request JSON in, response JSON out. This is the
// engine's wire protocol for every non-C++ consumer (wasm browser build,
// agent-driven CLI). Game state travels inside the request/response using the
// same save-file schema as Game::to_json/from_json.
//
// Request:  {"cmd": "...", ...args}   Commands taking a game carry "game": {...}.
//   new         difficulty?, seed?          -> game
//   load        game                        -> game (validated round-trip)
//   put         game, i, v                  -> game
//   mark        game, i, v                  -> game (toggle pencil mark)
//   clear_marks game, i                     -> game
//   hint        game, seed?                 -> game, index (null when solved)
//   check       game                        -> game, wrong: [indices]
//   candidates  game                        -> game, candidates: 81 digit lists
//   phantom     game, seed?                 -> game (see phantom_of)
//   grade       puzzle: [81 ints]           -> difficulty (no game/solved fields)
// Response: {"ok": true, "game": {...}, "solved": bool, ...extras}
//           {"ok": false, "error": "..."}   — never throws.
#pragma once

#include <string>

namespace souvenir {

std::string apply_command(const std::string &request);

} // namespace souvenir
