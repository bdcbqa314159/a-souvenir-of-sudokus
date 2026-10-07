// Fuzz harness for the one wire surface: souvenir_cmd(requestJson).
// node fuzz_api.mjs [iterations] [seed]
// Invariant under ANY input (the game JSON is user-editable localStorage):
//   - the module never aborts (an uncaught C++ exception kills wasm)
//   - the response is valid JSON with either ok:true or an error string
//   - an ok:true response carries a structurally sane game (81-cell arrays,
//     values 0..9) when it carries a game at all
import createSouvenir from "./engine/build/wasm/souvenir.js";

const ITER = parseInt(process.argv[2] ?? "20000", 10);
let s = BigInt(parseInt(process.argv[3] ?? "7", 10));
const rnd = () => {
  // splitmix64, deterministic across runs
  s += 0x9e3779b97f4a7c15n;
  let z = s;
  z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & 0xffffffffffffffffn;
  z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & 0xffffffffffffffffn;
  return Number((z ^ (z >> 31n)) & 0xfffffffn) / 0xfffffff;
};
const pick = (a) => a[Math.floor(rnd() * a.length)];
const irnd = (n) => Math.floor(rnd() * n);

const mod = await createSouvenir();
const cmd = mod.cwrap("souvenir_cmd", "string", ["string"]);

const base = JSON.parse(cmd(JSON.stringify({ cmd: "new", difficulty: "easy", seed: 1 }))).game;
const CMDS = ["new", "load", "put", "mark", "clear_marks", "hint", "check", "candidates", "phantom", "grade", "nope", "", 42];

function mutateGame() {
  const g = JSON.parse(JSON.stringify(base));
  const n = 1 + irnd(4);
  for (let k = 0; k < n; k++) {
    switch (irnd(12)) {
      case 0: if (Array.isArray(g.board)) g.board[irnd(81)] = pick([-1, 10, 255, 1e18, -(2 ** 40), 3.5, null, "7", [], {}]); break;
      case 1: if (Array.isArray(g.board)) g.board = g.board.slice(0, irnd(81)); break;
      case 2: delete g[pick(["board", "puzzle", "solution", "marks", "difficulty"])]; break;
      case 3: if (Array.isArray(g.solution)) g.solution[irnd(81)] = irnd(20) - 5; break;
      case 4: g.marks = pick([null, 7, "x", [], [[1, 2, 3]], Array(81).fill(pick([null, 9, "y", [99, -1]]))]); break;
      case 5: if (Array.isArray(g.puzzle)) g.puzzle[irnd(81)] = pick([9, 0, -3, 1e9]); break;
      case 6: g.difficulty = pick(["", "IMPOSSIBLE", 7, null, "easy".repeat(1000)]); break;
      case 7: g.board = Array(irnd(200)).fill(irnd(10)); break;
      case 8: g[`junk${irnd(5)}`] = { deep: { deeper: [1, 2, 3] } }; break;
      case 9: if (Array.isArray(g.solution)) g.board = g.solution.slice(); break; // solved-looking
      case 10: g.puzzle = g.board = g.solution = Array(81).fill(irnd(2) ? 0 : 9); break;
      case 11: g.solution = base.puzzle.slice(); break; // "solution" full of zeros
    }
  }
  return g;
}

function request() {
  const c = pick(CMDS);
  const r = { cmd: c };
  if (rnd() < 0.9) r.game = rnd() < 0.25 ? pick([null, 7, "game", [], { board: "nope" }]) : mutateGame();
  if (rnd() < 0.7) r.i = pick([irnd(81), -1, 81, 1e18, -(2 ** 33), 3.7, "4", null]);
  if (rnd() < 0.7) r.v = pick([irnd(10), -1, 10, 999, 1e18, "9", null, 2.5]);
  if (rnd() < 0.3) r.seed = pick([0, -1, 2 ** 63, 1.5, "s", null]);
  if (rnd() < 0.2) r.difficulty = pick(["easy", "medium", "hard", "x", 9, null]);
  return r;
}

let errors = 0;
for (let it = 0; it < ITER; it++) {
  const req = request();
  let raw;
  try {
    raw = cmd(JSON.stringify(req));
  } catch (e) {
    console.error(`ABORT at iter ${it}: ${e}\nrequest: ${JSON.stringify(req).slice(0, 400)}`);
    process.exit(1);
  }
  let rsp;
  try {
    rsp = JSON.parse(raw);
  } catch {
    console.error(`NON-JSON RESPONSE at iter ${it}: ${String(raw).slice(0, 200)}\nrequest: ${JSON.stringify(req).slice(0, 400)}`);
    process.exit(1);
  }
  if (rsp.ok !== true) {
    errors++;
    if (typeof rsp.error !== "string" || !rsp.error.length) {
      console.error(`ok!=true WITHOUT error string at iter ${it}: ${raw.slice(0, 200)}\nrequest: ${JSON.stringify(req).slice(0, 400)}`);
      process.exit(1);
    }
    continue;
  }
  const g = rsp.game;
  if (g !== undefined) {
    const arrs = [g.board, g.puzzle, g.solution];
    for (const a of arrs) {
      if (!Array.isArray(a) || a.length !== 81 || a.some((v) => !Number.isInteger(v) || v < 0 || v > 9)) {
        console.error(`MALFORMED GAME in ok:true at iter ${it}\nrequest: ${JSON.stringify(req).slice(0, 400)}\ngame: ${JSON.stringify(g).slice(0, 300)}`);
        process.exit(1);
      }
    }
  }
}
console.log(`PASS: ${ITER} hostile requests, 0 aborts, 0 non-JSON, 0 malformed ok-games (${errors} clean refusals)`);

// hint-spam: the dev solve button in the worst case — hint on a game whose
// "solution" disagrees with its givens must terminate and stay sane
let g2 = JSON.parse(JSON.stringify(base));
g2.solution[0] = ((g2.solution[0] % 9) + 1);
for (let k = 0; k < 200; k++) {
  const r = JSON.parse(cmd(JSON.stringify({ cmd: "hint", game: g2 })));
  if (r.ok !== true) break;
  g2 = r.game;
}
console.log("PASS: 200x hint-spam on an inconsistent game terminated without abort");
