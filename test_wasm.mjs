// Smoke test for the wasm build: node test_wasm.mjs
// Requires engine/build/wasm/souvenir.js (see README "Browser engine").
import createSouvenir from "./engine/build/wasm/souvenir.js";

const assert = (cond, msg) => {
  if (!cond) {
    console.error(`FAIL: ${msg}`);
    process.exit(1);
  }
};

const mod = await createSouvenir();
const cmd = mod.cwrap("souvenir_cmd", "string", ["string"]);
const call = (req) => JSON.parse(cmd(JSON.stringify(req)));

let rsp = call({ cmd: "new", difficulty: "easy", seed: 42 });
assert(rsp.ok && !rsp.solved, "new game");
const game = rsp.game;
assert(game.puzzle.length === 81 && game.solution.length === 81, "game shape");

const i = game.puzzle.findIndex((v) => v === 0);
rsp = call({ cmd: "put", game, i, v: game.solution[i] });
assert(rsp.ok && rsp.game.board[i] === game.solution[i], "put");

rsp = call({ cmd: "check", game: rsp.game });
assert(rsp.ok && rsp.wrong.length === 0, "check");

rsp = call({ cmd: "hint", game, seed: 1 });
assert(rsp.ok && Number.isInteger(rsp.index), "hint");

rsp = call({ cmd: "phantom", game, seed: 3 });
assert(rsp.ok && rsp.game.difficulty === "easy", "phantom");

rsp = call({ cmd: "put", game, i: 0, v: 99 });
assert(!rsp.ok && typeof rsp.error === "string", "error path");

// exceptions inside the engine must not corrupt later calls
rsp = call({ cmd: "new", seed: 7 });
assert(rsp.ok, "engine alive after error");

console.log("all wasm checks passed");
