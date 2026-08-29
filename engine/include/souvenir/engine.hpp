// a-souvenir-of-sudokus — C++ engine.
// Pure logic, no I/O. Board = 81 ints, 0 = empty, row-major.
// The JSON form of Game is the frontend contract, semantically interchangeable
// with the Python spec engine (sudoku.py): same schema and values, either
// engine loads the other's saves. Key order and whitespace may differ.
#pragma once

#include <array>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace souvenir {

inline constexpr int kCells = 81;
using Board = std::array<int, kCells>; // 0 = empty

// Peer cells (same row/column/box) of every cell.
const std::array<std::vector<int>, kCells> &peers();

// No digit repeated within any row/column/box.
bool consistent(const Board &board);

// A solution to `board`, or nullopt. A seed shuffles branching (used by generate).
std::optional<Board> solve(const Board &board, std::optional<std::uint64_t> seed = std::nullopt);

// Number of solutions, stopping at `limit` (uniqueness check = limit 2).
int count_solutions(const Board &board, int limit = 2);

enum class Difficulty { kEasy, kMedium, kHard };
Difficulty difficulty_from_string(const std::string &name); // throws std::invalid_argument
std::string to_string(Difficulty difficulty);
int clue_target(Difficulty difficulty);

struct Generated {
  Board puzzle;
  Board solution;
};

// Human-technique grader ("GM pass"): rates how a person solves it, not the
// backtracker. easy = singles suffice; medium = needs locked candidates or
// naked pairs; hard = beyond that ladder. Inconsistent input grades hard.
Difficulty grade(const Board &puzzle);

// Puzzle always has exactly one solution, and grade(puzzle) == difficulty
// (best effort: regenerates until the grade matches, capped at 200 attempts).
Generated generate(Difficulty difficulty, std::optional<std::uint64_t> seed = std::nullopt);

// Dig toward an explicit clue count (17..81, throws outside); the result may hold
// more clues than asked when digging further would break solution uniqueness.
Generated generate_with_clues(int clue_target, std::optional<std::uint64_t> seed = std::nullopt);

// One playing session. Errors are std::invalid_argument, mirroring ValueError in Python.
class Game {
public:
  Game(const Board &puzzle, const Board &solution, Difficulty difficulty = Difficulty::kMedium);

  static Game new_game(Difficulty difficulty = Difficulty::kMedium,
                       std::optional<std::uint64_t> seed = std::nullopt);

  const Board &puzzle() const { return puzzle_; }
  const Board &solution() const { return solution_; }
  const Board &board() const { return board_; }
  Difficulty difficulty() const { return difficulty_; }

  bool is_given(int i) const {
    if (i < 0 || i >= kCells)
      throw std::invalid_argument("cell index out of range");
    return puzzle_[static_cast<std::size_t>(i)] != 0;
  }
  void put(int i, int v);         // v = 0 clears; erases the cell's marks, prunes peer marks
  void toggle_mark(int i, int v); // pencil mark, only on empty non-given cells
  void clear_marks(int i);        // erase every pencil mark in one cell
  std::uint16_t marks(int i) const {
    return marks_[static_cast<std::size_t>(i)]; // bit d set = digit d marked
  }

  // Wholesale state replacement — frontend undo/restore support. Values are
  // range-validated but game invariants (givens, mark rules) are the caller's.
  void set_board(const Board &board);
  void set_marks(const std::array<std::uint16_t, kCells> &marks);

  std::vector<int> wrong_cells() const; // filled cells that contradict the solution
  bool is_solved() const { return board_ == solution_; }

  // Fill one empty-or-wrong cell with its solution value; returns the index.
  std::optional<int> hint(std::optional<std::uint64_t> seed = std::nullopt);

  std::string to_json() const;
  static Game from_json(const std::string &text); // validates; throws std::invalid_argument

private:
  void apply(int i, int v);

  Board puzzle_;
  Board solution_;
  Board board_;
  std::array<std::uint16_t, kCells> marks_{};
  Difficulty difficulty_;
};

// The phantom flip: a fresh puzzle whose given count equals the game's currently
// correct cell count (givens + correct entries), so the player's coverage carries
// over. Counts below 17 are clamped up — gameplay must go on. Difficulty label is
// preserved; board resets to the new givens, marks cleared.
Game phantom_of(const Game &game, std::optional<std::uint64_t> seed = std::nullopt);

} // namespace souvenir
