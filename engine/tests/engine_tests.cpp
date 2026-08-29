#include <gtest/gtest.h>

#include <algorithm>
#include <nlohmann/json.hpp>
#include <souvenir/api.hpp>
#include <souvenir/engine.hpp>

using souvenir::Board;
using souvenir::Difficulty;
using souvenir::Game;
using souvenir::kCells;

namespace {

int clue_count(const Board &b) {
  return static_cast<int>(std::count_if(b.begin(), b.end(), [](int v) { return v != 0; }));
}

bool valid_full_grid(const Board &b) {
  return std::find(b.begin(), b.end(), 0) == b.end() && souvenir::consistent(b);
}

} // namespace

TEST(Solve, EmptyBoardYieldsValidGrid) {
  auto full = souvenir::solve(Board{}, 1);
  ASSERT_TRUE(full.has_value());
  EXPECT_TRUE(valid_full_grid(*full));
}

TEST(Solve, InconsistentBoardRejected) {
  Board bad{};
  bad[0] = bad[1] = 5;
  EXPECT_FALSE(souvenir::solve(bad).has_value());
  EXPECT_EQ(souvenir::count_solutions(bad), 0);
}

TEST(Generate, UniqueSolutionAndClueTargets) {
  const std::pair<Difficulty, int> cases[] = {
      {Difficulty::kEasy, 40}, {Difficulty::kMedium, 26}, {Difficulty::kHard, 24}};
  for (auto [diff, target] : cases) {
    auto g = souvenir::generate(diff, 42);
    EXPECT_TRUE(valid_full_grid(g.solution));
    EXPECT_EQ(souvenir::count_solutions(g.puzzle), 1);
    EXPECT_GE(clue_count(g.puzzle), target);
    for (int i = 0; i < kCells; ++i) {
      auto u = static_cast<std::size_t>(i);
      EXPECT_TRUE(g.puzzle[u] == 0 || g.puzzle[u] == g.solution[u]);
    }
    auto solved = souvenir::solve(g.puzzle);
    ASSERT_TRUE(solved.has_value());
    EXPECT_EQ(*solved, g.solution);
  }
}

TEST(Generate, SeedReproducible) {
  auto a = souvenir::generate(Difficulty::kMedium, 7);
  auto b = souvenir::generate(Difficulty::kMedium, 7);
  EXPECT_EQ(a.puzzle, b.puzzle);
  EXPECT_EQ(a.solution, b.solution);
}

TEST(Generate, ExplicitClueTarget) {
  for (int target : {17, 30, 55}) {
    auto g = souvenir::generate_with_clues(target, 9);
    EXPECT_EQ(souvenir::count_solutions(g.puzzle), 1);
    EXPECT_GE(clue_count(g.puzzle), target);
    EXPECT_EQ(souvenir::solve(g.puzzle), g.solution);
  }
  auto full = souvenir::generate_with_clues(81, 9);
  EXPECT_EQ(full.puzzle, full.solution);
  EXPECT_THROW(souvenir::generate_with_clues(16, 9), std::invalid_argument);
  EXPECT_THROW(souvenir::generate_with_clues(82, 9), std::invalid_argument);
}

TEST(Grade, GeneratedPuzzlesMatchTheirLabel) {
  for (auto d : {Difficulty::kEasy, Difficulty::kMedium, Difficulty::kHard})
    for (std::uint64_t seed : {1ULL, 2ULL, 3ULL}) {
      auto g = souvenir::generate(d, seed);
      EXPECT_EQ(souvenir::grade(g.puzzle), d) << souvenir::to_string(d) << " seed " << seed;
      EXPECT_EQ(souvenir::count_solutions(g.puzzle), 1);
    }
}

TEST(Grade, Degenerates) {
  auto full = souvenir::generate_with_clues(81, 1);
  EXPECT_EQ(souvenir::grade(full.puzzle), Difficulty::kEasy); // nothing to do
  Board bad{};
  bad[0] = bad[1] = 5;
  EXPECT_EQ(souvenir::grade(bad), Difficulty::kHard); // sparse inconsistent -> hard
  Board bad_full = full.puzzle;
  bad_full[0] = bad_full[1]; // FULL inconsistent board must also grade hard
  EXPECT_EQ(souvenir::grade(bad_full), Difficulty::kHard);
}

TEST(Phantom, PreservesCoverage) {
  Game g = Game::new_game(Difficulty::kMedium, 4);
  // fill five empty cells correctly, one wrongly
  int placed = 0;
  for (int i = 0; i < kCells && placed < 6; ++i)
    if (!g.is_given(i) && g.board()[static_cast<std::size_t>(i)] == 0) {
      int right = g.solution()[static_cast<std::size_t>(i)];
      g.put(i, placed < 5 ? right : (right % 9) + 1); // 6th entry is wrong
      ++placed;
    }
  int correct = 0;
  for (int i = 0; i < kCells; ++i) {
    auto u = static_cast<std::size_t>(i);
    if (g.board()[u] != 0 && g.board()[u] == g.solution()[u])
      ++correct;
  }
  Game p = souvenir::phantom_of(g, 7);
  EXPECT_GE(clue_count(p.puzzle()), correct); // >=: uniqueness may block digging
  EXPECT_EQ(souvenir::count_solutions(p.puzzle()), 1);
  EXPECT_EQ(p.board(), p.puzzle()); // fresh board, wrong entry gone
  EXPECT_EQ(p.difficulty(), g.difficulty());
  for (int i = 0; i < kCells; ++i)
    EXPECT_EQ(p.marks(i), 0u);
}

TEST(Api, CommandRoundTrip) {
  using nlohmann::json;
  auto call = [](json req) { return json::parse(souvenir::apply_command(req.dump())); };

  json rsp = call({{"cmd", "new"}, {"difficulty", "easy"}, {"seed", 42}});
  ASSERT_TRUE(rsp["ok"].get<bool>());
  EXPECT_FALSE(rsp["solved"].get<bool>());
  json game = rsp["game"];

  int empty = 0;
  while (game["puzzle"][static_cast<std::size_t>(empty)].get<int>() != 0)
    ++empty;
  int right = game["solution"][static_cast<std::size_t>(empty)].get<int>();
  rsp = call({{"cmd", "put"}, {"game", game}, {"i", empty}, {"v", right}});
  ASSERT_TRUE(rsp["ok"].get<bool>());
  EXPECT_EQ(rsp["game"]["board"][static_cast<std::size_t>(empty)].get<int>(), right);

  rsp = call({{"cmd", "check"}, {"game", rsp["game"]}});
  EXPECT_TRUE(rsp["wrong"].empty());

  rsp = call({{"cmd", "hint"}, {"game", game}, {"seed", 1}});
  ASSERT_TRUE(rsp["ok"].get<bool>());
  EXPECT_TRUE(rsp["index"].is_number_integer());

  rsp = call({{"cmd", "candidates"}, {"game", game}});
  ASSERT_TRUE(rsp["ok"].get<bool>());
  EXPECT_EQ(rsp["candidates"].size(), 81u);
  for (int d : rsp["candidates"][static_cast<std::size_t>(empty)])
    EXPECT_TRUE(1 <= d && d <= 9);

  rsp = call({{"cmd", "phantom"}, {"game", game}, {"seed", 3}});
  ASSERT_TRUE(rsp["ok"].get<bool>());
  EXPECT_EQ(rsp["game"]["difficulty"], "easy");
}

TEST(Api, ErrorsNeverThrow) {
  using nlohmann::json;
  for (const std::string &req :
       {std::string("not json"), std::string("{}"), json{{"cmd", "fly"}}.dump(),
        json{{"cmd", "put"}, {"i", 0}, {"v", 1}}.dump(),       // missing game
        json{{"cmd", "new"}, {"difficulty", "expert"}}.dump(), // bad difficulty
        json{{"cmd", "new"}, {"seed", -4}}.dump()}) {          // negative seed
    json rsp = json::parse(souvenir::apply_command(req));
    EXPECT_FALSE(rsp["ok"].get<bool>()) << req;
    EXPECT_TRUE(rsp["error"].is_string()) << req;
  }
  // put on a given surfaces as an error response, not an exception
  json game =
      json::parse(souvenir::apply_command(json{{"cmd", "new"}, {"seed", 42}}.dump()))["game"];
  int given = 0;
  while (game["puzzle"][static_cast<std::size_t>(given)].get<int>() == 0)
    ++given;
  json rsp = json::parse(
      souvenir::apply_command(json{{"cmd", "put"}, {"game", game}, {"i", given}, {"v", 5}}.dump()));
  EXPECT_FALSE(rsp["ok"].get<bool>());
}

TEST(DifficultyNames, RoundTripAndRejection) {
  for (auto d : {Difficulty::kEasy, Difficulty::kMedium, Difficulty::kHard})
    EXPECT_EQ(souvenir::difficulty_from_string(souvenir::to_string(d)), d);
  EXPECT_THROW(souvenir::difficulty_from_string("expert"), std::invalid_argument);
}

TEST(GameFlow, PutGivensMarksHint) {
  Game g = Game::new_game(Difficulty::kEasy, 3);
  int empty = -1, given = -1;
  for (int i = 0; i < kCells && (empty < 0 || given < 0); ++i)
    (g.is_given(i) ? given : empty) = i;
  ASSERT_GE(empty, 0);
  ASSERT_GE(given, 0);

  EXPECT_THROW(g.put(given, 5), std::invalid_argument);
  EXPECT_THROW(g.put(-1, 5), std::invalid_argument);
  EXPECT_THROW(g.toggle_mark(given, 1), std::invalid_argument);
  EXPECT_THROW(g.is_given(-1), std::invalid_argument);
  EXPECT_THROW(g.is_given(81), std::invalid_argument);
  EXPECT_THROW(g.marks(-1), std::invalid_argument);
  EXPECT_THROW(g.marks(81), std::invalid_argument);

  g.toggle_mark(empty, 7);
  g.toggle_mark(empty, 3);
  g.toggle_mark(empty, 7);
  EXPECT_EQ(g.marks(empty), 1u << 3);

  // placing a value clears the cell's marks and prunes the digit from a peer's marks
  int peer = -1;
  for (int i = empty + 1; i < kCells; ++i)
    if (!g.is_given(i) && g.board()[static_cast<std::size_t>(i)] == 0 &&
        (i / 9 == empty / 9 || i % 9 == empty % 9)) {
      peer = i;
      break;
    }
  ASSERT_GE(peer, 0);
  g.toggle_mark(peer, 3);
  g.toggle_mark(peer, 5);
  g.put(empty, 3);
  EXPECT_EQ(g.marks(empty), 0u);
  EXPECT_EQ(g.marks(peer), 1u << 5);

  g.put(empty, 0);
  EXPECT_TRUE(g.wrong_cells().empty());
  while (g.hint(1).has_value()) {
  }
  EXPECT_TRUE(g.is_solved());
}

TEST(GameFlow, StateRestoreAndClearMarks) {
  Game g = Game::new_game(Difficulty::kEasy, 3);
  int empty = 0;
  while (g.is_given(empty))
    ++empty;
  g.toggle_mark(empty, 4);
  g.clear_marks(empty);
  EXPECT_EQ(g.marks(empty), 0u);

  Board board = g.board();
  std::array<std::uint16_t, kCells> marks{};
  marks[static_cast<std::size_t>(empty)] = 1u << 6;
  g.set_board(board);
  g.set_marks(marks);
  EXPECT_EQ(g.marks(empty), 1u << 6);

  Board bad = board;
  bad[0] = 12;
  EXPECT_THROW(g.set_board(bad), std::invalid_argument);
  marks[0] = 1; // bit 0 is not a digit
  EXPECT_THROW(g.set_marks(marks), std::invalid_argument);
  EXPECT_THROW(g.clear_marks(81), std::invalid_argument);
}

TEST(Peers, TopologyExposed) {
  const auto &p = souvenir::peers();
  for (int i = 0; i < kCells; ++i)
    EXPECT_EQ(p[static_cast<std::size_t>(i)].size(), 20u);
  // cell 0 sees its row, column and box
  EXPECT_NE(std::find(p[0].begin(), p[0].end(), 8), p[0].end());  // row
  EXPECT_NE(std::find(p[0].begin(), p[0].end(), 72), p[0].end()); // column
  EXPECT_NE(std::find(p[0].begin(), p[0].end(), 20), p[0].end()); // box
}

TEST(Json, RoundTripPreservesEverything) {
  Game g = Game::new_game(Difficulty::kHard, 11);
  int empty = 0;
  while (g.is_given(empty))
    ++empty;
  g.toggle_mark(empty, 2);
  g.toggle_mark(empty, 9);
  Game h = Game::from_json(g.to_json());
  EXPECT_EQ(h.board(), g.board());
  EXPECT_EQ(h.puzzle(), g.puzzle());
  EXPECT_EQ(h.solution(), g.solution());
  EXPECT_EQ(h.difficulty(), g.difficulty());
  for (int i = 0; i < kCells; ++i)
    EXPECT_EQ(h.marks(i), g.marks(i));
}

TEST(Json, MatchesPythonSchema) {
  Game g = Game::new_game(Difficulty::kEasy, 5);
  auto j = nlohmann::json::parse(g.to_json());
  EXPECT_EQ(j["name"], "a-souvenir-of-sudokus");
  EXPECT_EQ(j["difficulty"], "easy");
  for (const char *key : {"puzzle", "solution", "board", "marks"})
    EXPECT_EQ(j[key].size(), static_cast<std::size_t>(kCells));
}

TEST(Json, MalformedRejected) {
  EXPECT_THROW(Game::from_json("not json"), std::invalid_argument);
  EXPECT_THROW(Game::from_json("{}"), std::invalid_argument);

  Game g = Game::new_game(Difficulty::kEasy, 5);
  auto j = nlohmann::json::parse(g.to_json());

  auto corrupt = [&](auto mutate) {
    auto c = j;
    mutate(c);
    EXPECT_THROW(Game::from_json(c.dump()), std::invalid_argument);
  };
  corrupt([](nlohmann::json &c) { c["board"].erase(80); });       // short grid
  corrupt([](nlohmann::json &c) { c["board"][0] = 12; });         // out-of-range value
  corrupt([](nlohmann::json &c) { c["solution"][0] = 0; });       // unsolved solution
  corrupt([](nlohmann::json &c) { c["marks"][0] = {0}; });        // bad mark digit
  corrupt([](nlohmann::json &c) { c["difficulty"] = "expert"; }); // unknown difficulty
  corrupt([](nlohmann::json &c) { c["difficulty"] = 5; });        // non-string difficulty
  // 64-bit values that would wrap into range if narrowed to int32 before checking
  corrupt([](nlohmann::json &c) { c["board"][0] = 4294967301ULL; });   // 2^32 + 5
  corrupt([](nlohmann::json &c) { c["solution"][0] = -4294967287; });  // -2^32 + 9
  corrupt([](nlohmann::json &c) { c["marks"][0] = {4294967299ULL}; }); // 2^32 + 3
  corrupt([](nlohmann::json &c) { // puzzle contradicts solution
    for (int i = 0; i < kCells; ++i) {
      int s = c["solution"][static_cast<std::size_t>(i)].get<int>();
      c["puzzle"][static_cast<std::size_t>(i)] = 10 - s;
    }
  });

  // fewer givens than the solution is still a valid save
  auto ok = j;
  for (int i = 0; i < kCells; ++i)
    if (ok["puzzle"][static_cast<std::size_t>(i)].get<int>() != 0) {
      ok["puzzle"][static_cast<std::size_t>(i)] = 0;
      break;
    }
  EXPECT_NO_THROW(Game::from_json(ok.dump()));
}
