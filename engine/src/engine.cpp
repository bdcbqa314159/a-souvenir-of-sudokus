#include "souvenir/engine.hpp"

#include <algorithm>
#include <bit>
#include <random>
#include <stdexcept>

#include <nlohmann/json.hpp>

namespace souvenir {
namespace {

constexpr int kDigitMask = 0b1111111110; // bits 1..9

struct Topology {
  std::array<std::vector<int>, 27> units; // 9 rows, 9 cols, 9 boxes
  std::array<std::vector<int>, kCells> peers;
};

const Topology &topology() {
  static const Topology t = [] {
    Topology t;
    for (int r = 0; r < 9; ++r)
      for (int c = 0; c < 9; ++c) {
        int i = r * 9 + c;
        t.units[static_cast<std::size_t>(r)].push_back(i);
        t.units[static_cast<std::size_t>(9 + c)].push_back(i);
        t.units[static_cast<std::size_t>(18 + (r / 3) * 3 + c / 3)].push_back(i);
      }
    for (const auto &unit : t.units)
      for (int a : unit)
        for (int b : unit)
          if (a != b) {
            auto &p = t.peers[static_cast<std::size_t>(a)];
            if (std::find(p.begin(), p.end(), b) == p.end())
              p.push_back(b);
          }
    return t;
  }();
  return t;
}

int candidate_mask(const Board &b, int i) {
  int used = 0;
  for (int p : topology().peers[static_cast<std::size_t>(i)])
    used |= 1 << b[static_cast<std::size_t>(p)];
  return ~used & kDigitMask;
}

// Empty cell with fewest candidates; -1 if the board is full.
int most_constrained(const Board &b, int &mask_out) {
  int best = -1, best_n = 10;
  for (int i = 0; i < kCells; ++i)
    if (b[static_cast<std::size_t>(i)] == 0) {
      int m = candidate_mask(b, i);
      int n = std::popcount(static_cast<unsigned>(m));
      if (n < best_n) {
        best = i;
        best_n = n;
        mask_out = m;
        if (n <= 1)
          break;
      }
    }
  return best;
}

bool backtrack(Board &b, std::mt19937_64 *rng) {
  int mask = 0;
  int i = most_constrained(b, mask);
  if (i < 0)
    return true;
  std::array<int, 9> vals{};
  int nv = 0;
  for (int d = 1; d <= 9; ++d)
    if ((mask >> d) & 1)
      vals[static_cast<std::size_t>(nv++)] = d;
  if (rng)
    std::shuffle(vals.begin(), vals.begin() + nv, *rng);
  for (int k = 0; k < nv; ++k) {
    b[static_cast<std::size_t>(i)] = vals[static_cast<std::size_t>(k)];
    if (backtrack(b, rng))
      return true;
  }
  b[static_cast<std::size_t>(i)] = 0;
  return false;
}

// Returns true when `limit` was reached, to abort the search.
bool count_backtrack(Board &b, int limit, int &n) {
  int mask = 0;
  int i = most_constrained(b, mask);
  if (i < 0)
    return ++n >= limit;
  for (int d = 1; d <= 9; ++d)
    if ((mask >> d) & 1) {
      b[static_cast<std::size_t>(i)] = d;
      if (count_backtrack(b, limit, n))
        return true;
    }
  b[static_cast<std::size_t>(i)] = 0;
  return false;
}

std::mt19937_64 make_rng(std::optional<std::uint64_t> seed) {
  return std::mt19937_64{seed ? *seed : std::random_device{}()};
}

} // namespace

const std::array<std::vector<int>, kCells> &peers() { return topology().peers; }

// Dig targets tuned so the grader's target grade is likely per attempt
// (measured: medium ~15% at 26 clues, hard ~40% at 24; at 32 clues medium
// was a 2% needle and generate()'s retry cap became reachable).
int clue_target(Difficulty difficulty) {
  switch (difficulty) {
  case Difficulty::kEasy:
    return 40;
  case Difficulty::kMedium:
    return 26;
  case Difficulty::kHard:
    return 24;
  }
  throw std::invalid_argument("unknown difficulty");
}

bool consistent(const Board &board) {
  for (const auto &unit : topology().units) {
    int seen = 0;
    for (int i : unit) {
      int v = board[static_cast<std::size_t>(i)];
      if (v == 0)
        continue;
      if (v < 1 || v > 9 || (seen >> v) & 1)
        return false;
      seen |= 1 << v;
    }
  }
  return true;
}

std::optional<Board> solve(const Board &board, std::optional<std::uint64_t> seed) {
  if (!consistent(board))
    return std::nullopt;
  Board b = board;
  if (seed) {
    std::mt19937_64 rng{*seed};
    if (!backtrack(b, &rng))
      return std::nullopt;
  } else {
    // no rng at all: unseeded solve is deterministic and must not touch
    // std::random_device (whose constructor may throw without an entropy source)
    if (!backtrack(b, nullptr))
      return std::nullopt;
  }
  return b;
}

int count_solutions(const Board &board, int limit) {
  if (!consistent(board))
    return 0;
  Board b = board;
  int n = 0;
  count_backtrack(b, limit, n);
  return n;
}

Difficulty difficulty_from_string(const std::string &name) {
  if (name == "easy")
    return Difficulty::kEasy;
  if (name == "medium")
    return Difficulty::kMedium;
  if (name == "hard")
    return Difficulty::kHard;
  throw std::invalid_argument("difficulty must be one of easy, medium, hard");
}

std::string to_string(Difficulty difficulty) {
  switch (difficulty) {
  case Difficulty::kEasy:
    return "easy";
  case Difficulty::kMedium:
    return "medium";
  case Difficulty::kHard:
    return "hard";
  }
  throw std::invalid_argument("unknown difficulty");
}

Generated generate_with_clues(int clue_target, std::optional<std::uint64_t> seed) {
  if (clue_target < 17 || clue_target > kCells) // 17 = minimum clues of any unique sudoku
    throw std::invalid_argument("clue target must be 17-81");
  auto rng = make_rng(seed);
  Board full{};
  backtrack(full, &rng); // empty board is consistent; always succeeds
  Board puzzle = full;
  std::array<int, kCells> order{};
  for (int i = 0; i < kCells; ++i)
    order[static_cast<std::size_t>(i)] = i;
  std::shuffle(order.begin(), order.end(), rng);
  int clues = kCells;
  for (int i : order) {
    if (clues <= clue_target)
      break;
    int saved = puzzle[static_cast<std::size_t>(i)];
    puzzle[static_cast<std::size_t>(i)] = 0;
    if (count_solutions(puzzle) == 1) {
      --clues;
    } else {
      puzzle[static_cast<std::size_t>(i)] = saved;
    }
  }
  return {puzzle, full};
}

Generated generate(Difficulty difficulty, std::optional<std::uint64_t> seed) {
  auto seeder = make_rng(seed);
  Generated g{};
  for (int attempt = 0; attempt < 200; ++attempt) {
    g = generate_with_clues(clue_target(difficulty), seeder());
    if (grade(g.puzzle) == difficulty)
      return g;
  }
  return g; // ponytail: 200 grade misses — serve the last one rather than spin forever
}

Game phantom_of(const Game &game, std::optional<std::uint64_t> seed) {
  int correct = 0;
  for (int i = 0; i < kCells; ++i) {
    auto u = static_cast<std::size_t>(i);
    if (game.board()[u] != 0 && game.board()[u] == game.solution()[u])
      ++correct;
  }
  Generated g = generate_with_clues(std::max(correct, 17), seed);
  return Game(g.puzzle, g.solution, game.difficulty());
}

Game::Game(const Board &puzzle, const Board &solution, Difficulty difficulty)
    : puzzle_(puzzle), solution_(solution), board_(puzzle), difficulty_(difficulty) {}

Game Game::new_game(Difficulty difficulty, std::optional<std::uint64_t> seed) {
  Generated g = generate(difficulty, seed);
  return Game(g.puzzle, g.solution, difficulty);
}

void Game::apply(int i, int v) {
  auto u = static_cast<std::size_t>(i);
  board_[u] = v;
  if (v != 0) { // a placed value erases the cell's marks and that digit from peer marks
    marks_[u] = 0;
    for (int p : topology().peers[u])
      marks_[static_cast<std::size_t>(p)] &= static_cast<std::uint16_t>(~(1u << v));
  }
}

void Game::put(int i, int v) {
  if (i < 0 || i >= kCells)
    throw std::invalid_argument("cell index out of range");
  if (is_given(i))
    throw std::invalid_argument("cell is a given");
  if (v < 0 || v > 9)
    throw std::invalid_argument("value must be 0-9");
  apply(i, v);
}

void Game::toggle_mark(int i, int v) {
  if (i < 0 || i >= kCells)
    throw std::invalid_argument("cell index out of range");
  if (is_given(i) || board_[static_cast<std::size_t>(i)] != 0)
    throw std::invalid_argument("cell not markable");
  if (v < 1 || v > 9)
    throw std::invalid_argument("mark must be 1-9");
  marks_[static_cast<std::size_t>(i)] ^= static_cast<std::uint16_t>(1u << v);
}

void Game::clear_marks(int i) {
  if (i < 0 || i >= kCells)
    throw std::invalid_argument("cell index out of range");
  marks_[static_cast<std::size_t>(i)] = 0;
}

void Game::set_board(const Board &board) {
  for (int v : board)
    if (v < 0 || v > 9)
      throw std::invalid_argument("value must be 0-9");
  board_ = board;
}

void Game::set_marks(const std::array<std::uint16_t, kCells> &marks) {
  for (auto m : marks)
    if ((m & ~static_cast<unsigned>(kDigitMask)) != 0)
      throw std::invalid_argument("marks must be 1-9");
  marks_ = marks;
}

std::vector<int> Game::wrong_cells() const {
  std::vector<int> out;
  for (int i = 0; i < kCells; ++i) {
    auto u = static_cast<std::size_t>(i);
    if (board_[u] != 0 && board_[u] != solution_[u])
      out.push_back(i);
  }
  return out;
}

std::optional<int> Game::hint(std::optional<std::uint64_t> seed) {
  std::vector<int> todo;
  for (int i = 0; i < kCells; ++i) {
    auto u = static_cast<std::size_t>(i);
    if (board_[u] != solution_[u])
      todo.push_back(i);
  }
  if (todo.empty())
    return std::nullopt;
  auto rng = make_rng(seed);
  int i = todo[std::uniform_int_distribution<std::size_t>{0, todo.size() - 1}(rng)];
  apply(i, solution_[static_cast<std::size_t>(i)]);
  return i;
}

std::string Game::to_json() const {
  nlohmann::json marks = nlohmann::json::array();
  for (std::uint16_t m : marks_) {
    nlohmann::json cell = nlohmann::json::array();
    for (int d = 1; d <= 9; ++d)
      if ((m >> d) & 1)
        cell.push_back(d);
    marks.push_back(std::move(cell));
  }
  nlohmann::json j{{"name", "a-souvenir-of-sudokus"},
                   {"difficulty", to_string(difficulty_)},
                   {"puzzle", puzzle_},
                   {"solution", solution_},
                   {"board", board_},
                   {"marks", std::move(marks)}};
  return j.dump();
}

Game Game::from_json(const std::string &text) {
  nlohmann::json j = nlohmann::json::parse(text, nullptr, /*allow_exceptions=*/false);
  if (j.is_discarded() || !j.is_object())
    throw std::invalid_argument("bad save file: not JSON");

  // Range-check on the wide type BEFORE narrowing — get<int>() would wrap
  // 2^32+5 to 5 and let malformed saves through. Values > INT64_MAX arrive
  // negative here (or as floats, already rejected) and fail the range check.
  auto digit = [](const nlohmann::json &v, int lo) -> int {
    if (!v.is_number_integer())
      return -1;
    const std::int64_t n = v.get<std::int64_t>();
    return (n < lo || n > 9) ? -1 : static_cast<int>(n);
  };

  auto grid = [&](const char *key) {
    Board out{};
    if (!j.contains(key) || !j[key].is_array() || j[key].size() != kCells)
      throw std::invalid_argument(std::string("bad save file: ") + key);
    for (int i = 0; i < kCells; ++i) {
      int v = digit(j[key][static_cast<std::size_t>(i)], 0);
      if (v < 0)
        throw std::invalid_argument(std::string("bad save file: ") + key);
      out[static_cast<std::size_t>(i)] = v;
    }
    return out;
  };

  Board puzzle = grid("puzzle");
  Board solution = grid("solution");
  Board board = grid("board");
  if (std::find(solution.begin(), solution.end(), 0) != solution.end() || !consistent(solution))
    throw std::invalid_argument("bad save file: solution is not a solved grid");
  for (int i = 0; i < kCells; ++i) {
    auto u = static_cast<std::size_t>(i);
    if (puzzle[u] != 0 && puzzle[u] != solution[u])
      throw std::invalid_argument("bad save file: puzzle/solution mismatch");
  }

  Difficulty d = Difficulty::kMedium;
  if (j.contains("difficulty")) {
    if (!j["difficulty"].is_string())
      throw std::invalid_argument("bad save file: difficulty");
    d = difficulty_from_string(j["difficulty"].get<std::string>());
  }
  Game g(puzzle, solution, d);
  g.board_ = board;
  if (j.contains("marks")) {
    if (!j["marks"].is_array() || j["marks"].size() != kCells)
      throw std::invalid_argument("bad save file: marks");
    for (int i = 0; i < kCells; ++i) {
      const auto &cell = j["marks"][static_cast<std::size_t>(i)];
      if (!cell.is_array())
        throw std::invalid_argument("bad save file: marks");
      for (const auto &v : cell) {
        int m = digit(v, 1);
        if (m < 0)
          throw std::invalid_argument("bad save file: marks");
        g.marks_[static_cast<std::size_t>(i)] |= static_cast<std::uint16_t>(1u << m);
      }
    }
  }
  return g;
}

} // namespace souvenir
