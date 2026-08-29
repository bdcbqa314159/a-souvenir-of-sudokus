#include "souvenir/api.hpp"

#include <cstdint>
#include <optional>
#include <stdexcept>

#include <nlohmann/json.hpp>

#include "souvenir/engine.hpp"

namespace souvenir {
namespace {

using nlohmann::json;

std::optional<std::uint64_t> seed_of(const json &req) {
  if (!req.contains("seed") || req["seed"].is_null())
    return std::nullopt;
  if (!req["seed"].is_number_integer())
    throw std::invalid_argument("seed must be an integer");
  if (req["seed"].is_number_unsigned())
    return req["seed"].get<std::uint64_t>();
  const std::int64_t n = req["seed"].get<std::int64_t>();
  if (n < 0)
    throw std::invalid_argument("seed must be non-negative");
  return static_cast<std::uint64_t>(n);
}

int int_arg(const json &req, const char *key) {
  if (!req.contains(key) || !req[key].is_number_integer())
    throw std::invalid_argument(std::string(key) + " must be an integer");
  const std::int64_t n = req[key].get<std::int64_t>();
  if (n < -1 || n > kCells) // engine re-validates exact ranges; this only blocks wrap
    throw std::invalid_argument(std::string(key) + " out of range");
  return static_cast<int>(n);
}

Game game_of(const json &req) {
  if (!req.contains("game"))
    throw std::invalid_argument("missing game");
  return Game::from_json(req["game"].dump());
}

json respond(const Game &g, json extras = json::object()) {
  json rsp{{"ok", true}, {"game", json::parse(g.to_json())}, {"solved", g.is_solved()}};
  rsp.update(extras);
  return rsp;
}

} // namespace

std::string apply_command(const std::string &request) {
  try {
    json req = json::parse(request, nullptr, /*allow_exceptions=*/false);
    if (req.is_discarded() || !req.is_object())
      throw std::invalid_argument("bad request: not JSON");
    if (!req.contains("cmd") || !req["cmd"].is_string())
      throw std::invalid_argument("missing cmd");
    const std::string cmd = req["cmd"].get<std::string>();

    if (cmd == "new") {
      const std::string diff = req.contains("difficulty") && req["difficulty"].is_string()
                                   ? req["difficulty"].get<std::string>()
                                   : "medium";
      return respond(Game::new_game(difficulty_from_string(diff), seed_of(req))).dump();
    }
    if (cmd == "load")
      return respond(game_of(req)).dump();
    if (cmd == "put") {
      Game g = game_of(req);
      g.put(int_arg(req, "i"), int_arg(req, "v"));
      return respond(g).dump();
    }
    if (cmd == "mark") {
      Game g = game_of(req);
      g.toggle_mark(int_arg(req, "i"), int_arg(req, "v"));
      return respond(g).dump();
    }
    if (cmd == "clear_marks") {
      Game g = game_of(req);
      g.clear_marks(int_arg(req, "i"));
      return respond(g).dump();
    }
    if (cmd == "hint") {
      Game g = game_of(req);
      auto i = g.hint(seed_of(req));
      return respond(g, {{"index", i ? json(*i) : json(nullptr)}}).dump();
    }
    if (cmd == "check") {
      Game g = game_of(req);
      return respond(g, {{"wrong", g.wrong_cells()}}).dump();
    }
    if (cmd == "candidates") {
      Game g = game_of(req);
      json cands = json::array();
      for (int i = 0; i < kCells; ++i) {
        json cell = json::array();
        if (g.board()[static_cast<std::size_t>(i)] == 0) {
          unsigned used = 0;
          for (int p : peers()[static_cast<std::size_t>(i)])
            used |= 1u << g.board()[static_cast<std::size_t>(p)];
          for (int d = 1; d <= 9; ++d)
            if (!((used >> d) & 1))
              cell.push_back(d);
        }
        cands.push_back(std::move(cell));
      }
      return respond(g, {{"candidates", std::move(cands)}}).dump();
    }
    if (cmd == "phantom")
      return respond(phantom_of(game_of(req), seed_of(req))).dump();
    if (cmd == "grade") {
      if (!req.contains("puzzle") || !req["puzzle"].is_array() || req["puzzle"].size() != kCells)
        throw std::invalid_argument("grade needs a puzzle of 81 cells");
      Board p{};
      for (int i = 0; i < kCells; ++i) {
        const auto &v = req["puzzle"][static_cast<std::size_t>(i)];
        if (!v.is_number_integer())
          throw std::invalid_argument("bad puzzle cell");
        const std::int64_t n = v.get<std::int64_t>();
        if (n < 0 || n > 9)
          throw std::invalid_argument("bad puzzle cell");
        p[static_cast<std::size_t>(i)] = static_cast<int>(n);
      }
      return json{{"ok", true}, {"difficulty", to_string(grade(p))}}.dump();
    }

    throw std::invalid_argument("unknown cmd: " + cmd);
  } catch (const std::exception &e) {
    return json{{"ok", false}, {"error", e.what()}}.dump();
  }
}

} // namespace souvenir
