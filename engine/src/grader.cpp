// Human-technique grader: the "GM engine" pass. The backtracker finds every
// puzzle trivial; this solver only uses techniques a person uses, in a ladder,
// and the puzzle's grade is the highest rung it was forced onto:
//   easy   — naked + hidden singles suffice
//   medium — needed locked candidates (pointing/claiming) or naked pairs
//   hard   — the ladder is not enough (chains, trial and error, ...)
#include <array>
#include <bit>

#include "souvenir/engine.hpp"

namespace souvenir {
namespace {

constexpr int kDigitMask = 0b1111111110;

// 27 units: 9 rows, 9 cols, 9 boxes (grader-local; engine.cpp keeps its own)
const std::array<std::array<int, 9>, 27> &units() {
  static const auto u = [] {
    std::array<std::array<int, 9>, 27> u{};
    std::array<int, 27> fill{};
    for (int r = 0; r < 9; ++r)
      for (int c = 0; c < 9; ++c) {
        int i = r * 9 + c;
        int box = 18 + (r / 3) * 3 + c / 3;
        u[static_cast<std::size_t>(r)]
         [static_cast<std::size_t>(fill[static_cast<std::size_t>(r)]++)] = i;
        u[static_cast<std::size_t>(9 + c)]
         [static_cast<std::size_t>(fill[static_cast<std::size_t>(9 + c)]++)] = i;
        u[static_cast<std::size_t>(box)]
         [static_cast<std::size_t>(fill[static_cast<std::size_t>(box)]++)] = i;
      }
    return u;
  }();
  return u;
}

struct HumanSolver {
  Board b;
  std::array<int, kCells> cand{};

  explicit HumanSolver(const Board &puzzle) : b(puzzle) {
    for (int i = 0; i < kCells; ++i)
      if (b[static_cast<std::size_t>(i)] == 0) {
        int used = 0;
        for (int p : peers()[static_cast<std::size_t>(i)])
          used |= 1 << b[static_cast<std::size_t>(p)];
        cand[static_cast<std::size_t>(i)] = ~used & kDigitMask;
      }
  }

  void place(int i, int v) {
    b[static_cast<std::size_t>(i)] = v;
    cand[static_cast<std::size_t>(i)] = 0;
    for (int p : peers()[static_cast<std::size_t>(i)])
      cand[static_cast<std::size_t>(p)] &= ~(1 << v);
  }

  bool solved() const {
    for (int v : b)
      if (v == 0)
        return false;
    return true;
  }

  bool stuck() const { // an empty cell with no candidates: not human-solvable state
    for (int i = 0; i < kCells; ++i)
      if (b[static_cast<std::size_t>(i)] == 0 && cand[static_cast<std::size_t>(i)] == 0)
        return true;
    return false;
  }

  bool naked_single() {
    for (int i = 0; i < kCells; ++i) {
      int m = cand[static_cast<std::size_t>(i)];
      if (b[static_cast<std::size_t>(i)] == 0 && std::popcount(static_cast<unsigned>(m)) == 1) {
        place(i, std::countr_zero(static_cast<unsigned>(m)));
        return true;
      }
    }
    return false;
  }

  bool hidden_single() {
    for (const auto &unit : units())
      for (int d = 1; d <= 9; ++d) {
        int spot = -1, n = 0;
        for (int i : unit)
          if (b[static_cast<std::size_t>(i)] == 0 &&
              ((cand[static_cast<std::size_t>(i)] >> d) & 1)) {
            spot = i;
            ++n;
          }
        if (n == 1) {
          place(spot, d);
          return true;
        }
      }
    return false;
  }

  // Locked candidates, both directions: a digit confined to one row/col within
  // a box eliminates it from the rest of that line (pointing), and a digit
  // confined to one box within a line eliminates it from the rest of the box
  // (claiming). Boxes are units 18..26; lines are 0..17.
  bool locked_candidates() {
    bool changed = false;
    const auto &u = units();
    for (int a = 0; a < 27 && !changed; ++a)
      for (int d = 1; d <= 9 && !changed; ++d) {
        long row_mask = 0, col_mask = 0, box_mask = 0;
        int n = 0, first = -1;
        for (int i : u[static_cast<std::size_t>(a)])
          if (b[static_cast<std::size_t>(i)] == 0 &&
              ((cand[static_cast<std::size_t>(i)] >> d) & 1)) {
            ++n;
            first = i;
            row_mask |= 1L << (i / 9);
            col_mask |= 1L << (i % 9);
            box_mask |= 1L << ((i / 27) * 3 + (i % 9) / 3);
          }
        if (n < 2)
          continue;        // 0: nothing; 1: that's a hidden single, not our job
        int confined = -1; // the unit index the digit is confined to, if any
        if (a >= 18) {     // box: confined to one row or one column?
          if (std::popcount(static_cast<unsigned long>(row_mask)) == 1)
            confined = first / 9;
          else if (std::popcount(static_cast<unsigned long>(col_mask)) == 1)
            confined = 9 + first % 9;
        } else { // line: confined to one box?
          if (std::popcount(static_cast<unsigned long>(box_mask)) == 1)
            confined = 18 + (first / 27) * 3 + (first % 9) / 3;
        }
        if (confined < 0)
          continue;
        for (int i : u[static_cast<std::size_t>(confined)]) {
          bool in_source = false;
          for (int s : u[static_cast<std::size_t>(a)])
            in_source |= (s == i);
          if (!in_source && b[static_cast<std::size_t>(i)] == 0 &&
              ((cand[static_cast<std::size_t>(i)] >> d) & 1)) {
            cand[static_cast<std::size_t>(i)] &= ~(1 << d);
            changed = true;
          }
        }
      }
    return changed;
  }

  bool naked_pair() {
    for (const auto &unit : units())
      for (int x = 0; x < 9; ++x) {
        int i = unit[static_cast<std::size_t>(x)];
        int m = cand[static_cast<std::size_t>(i)];
        if (b[static_cast<std::size_t>(i)] != 0 || std::popcount(static_cast<unsigned>(m)) != 2)
          continue;
        for (int y = x + 1; y < 9; ++y) {
          int j = unit[static_cast<std::size_t>(y)];
          if (cand[static_cast<std::size_t>(j)] != m)
            continue;
          bool changed = false;
          for (int k : unit)
            if (k != i && k != j && b[static_cast<std::size_t>(k)] == 0 &&
                (cand[static_cast<std::size_t>(k)] & m)) {
              cand[static_cast<std::size_t>(k)] &= ~m;
              changed = true;
            }
          if (changed)
            return true;
        }
      }
    return false;
  }
};

} // namespace

Difficulty grade(const Board &puzzle) {
  HumanSolver s(puzzle);
  bool needed_eliminations = false;
  while (!s.solved()) {
    if (s.stuck())
      return Difficulty::kHard; // inconsistent input also lands here
    if (s.naked_single() || s.hidden_single())
      continue;
    if (s.locked_candidates() || s.naked_pair()) {
      needed_eliminations = true;
      continue;
    }
    return Difficulty::kHard; // beyond the human ladder
  }
  return needed_eliminations ? Difficulty::kMedium : Difficulty::kEasy;
}

} // namespace souvenir
