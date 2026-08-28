// Python bindings for the souvenir engine. The surface mirrors sudoku.py
// (the retired Python draft) so cli.py/tui.py work unchanged in spirit:
// board/puzzle/solution are lists, marks is a list of sets, errors are ValueError.
#include <pybind11/pybind11.h>
#include <pybind11/stl.h>

#include <souvenir/api.hpp>
#include <souvenir/engine.hpp>

namespace py = pybind11;

using souvenir::Board;
using souvenir::Game;
using souvenir::kCells;

PYBIND11_MODULE(souvenir, m) {
  m.doc() = "a-souvenir-of-sudokus — C++ engine";

  m.def("consistent", &souvenir::consistent, py::arg("board"));
  m.def(
      "solve",
      [](const Board &b, std::optional<std::uint64_t> seed) { return souvenir::solve(b, seed); },
      py::arg("board"), py::arg("seed") = py::none());
  m.def("count_solutions", &souvenir::count_solutions, py::arg("board"), py::arg("limit") = 2);
  m.def("apply_command", &souvenir::apply_command, py::arg("request"));
  m.def(
      "generate_with_clues",
      [](int clues, std::optional<std::uint64_t> seed) {
        auto g = souvenir::generate_with_clues(clues, seed);
        return py::make_tuple(g.puzzle, g.solution);
      },
      py::arg("clue_target"), py::arg("seed") = py::none());
  m.def(
      "phantom_of",
      [](const Game &g, std::optional<std::uint64_t> seed) {
        return souvenir::phantom_of(g, seed);
      },
      py::arg("game"), py::arg("seed") = py::none());
  m.def(
      "generate",
      [](const std::string &difficulty, std::optional<std::uint64_t> seed) {
        auto g = souvenir::generate(souvenir::difficulty_from_string(difficulty), seed);
        return py::make_tuple(g.puzzle, g.solution);
      },
      py::arg("difficulty") = "medium", py::arg("seed") = py::none());

  py::dict clue_target;
  for (auto d :
       {souvenir::Difficulty::kEasy, souvenir::Difficulty::kMedium, souvenir::Difficulty::kHard})
    clue_target[py::str(souvenir::to_string(d))] = souvenir::clue_target(d);
  m.attr("CLUE_TARGET") = clue_target;

  py::list peers;
  for (int i = 0; i < kCells; ++i) {
    py::set s;
    for (int p : souvenir::peers()[static_cast<std::size_t>(i)])
      s.add(p);
    peers.append(std::move(s));
  }
  m.attr("PEERS") = peers;

  py::class_<Game>(m, "Game")
      .def_static(
          "new",
          [](const std::string &difficulty, std::optional<std::uint64_t> seed) {
            return Game::new_game(souvenir::difficulty_from_string(difficulty), seed);
          },
          py::arg("difficulty") = "medium", py::arg("seed") = py::none())
      .def_static("from_json", &Game::from_json, py::arg("text"))
      .def("is_given", &Game::is_given, py::arg("i"))
      .def("put", &Game::put, py::arg("i"), py::arg("v"))
      .def("toggle_mark", &Game::toggle_mark, py::arg("i"), py::arg("v"))
      .def("clear_marks", &Game::clear_marks, py::arg("i"))
      .def("wrong_cells", &Game::wrong_cells)
      .def("is_solved", &Game::is_solved)
      .def(
          "hint", [](Game &g, std::optional<std::uint64_t> seed) { return g.hint(seed); },
          py::arg("seed") = py::none())
      .def("to_json", &Game::to_json)
      .def_property_readonly("puzzle", [](const Game &g) { return g.puzzle(); })
      .def_property_readonly("solution", [](const Game &g) { return g.solution(); })
      .def_property_readonly("difficulty",
                             [](const Game &g) { return souvenir::to_string(g.difficulty()); })
      .def_property(
          "board", [](const Game &g) { return g.board(); },
          [](Game &g, const Board &b) { g.set_board(b); })
      .def_property(
          "marks",
          [](const Game &g) {
            py::list out;
            for (int i = 0; i < kCells; ++i) {
              py::set s;
              for (int d = 1; d <= 9; ++d)
                if ((g.marks(i) >> d) & 1)
                  s.add(d);
              out.append(std::move(s));
            }
            return out;
          },
          [](Game &g, const std::vector<std::set<int>> &marks) {
            if (marks.size() != kCells)
              throw std::invalid_argument("marks must have 81 entries");
            std::array<std::uint16_t, kCells> m{};
            for (std::size_t i = 0; i < kCells; ++i)
              for (int d : marks[i]) {
                if (d < 1 || d > 9)
                  throw std::invalid_argument("mark must be 1-9");
                m[i] |= static_cast<std::uint16_t>(1u << d);
              }
            g.set_marks(m);
          });
}
