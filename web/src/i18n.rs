//! Every user-facing string, in the three languages, plus language
//! detection and persistence. "el abuelo" stays el abuelo everywhere.
use leptos::prelude::window;

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Classic,
    Phantom,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Lang {
    En,
    Fr,
    Es,
}

/// Every user-facing string, per language. "el abuelo" is who he is to
/// everyone — it stays el abuelo in all three languages.
pub struct T {
    pub pencil: &'static str,
    pub hint: &'static str,
    pub check: &'static str,
    pub undo: &'static str,
    pub redo: &'static str,
    pub new_game: &'static str,
    pub phantom: &'static str,
    pub easy: &'static str,
    pub medium: &'static str,
    pub hard: &'static str,
    pub pen: &'static str,
    pub pencil_mode: &'static str,
    pub loading: &'static str,
    pub engine_loading: &'static str,
    pub solved: &'static str,
    pub all_good: &'static str,
    pub wrong_cells: &'static str,   // {} count
    pub new_game_msg: &'static str,  // {} difficulty label
    pub hints_noun: &'static str,
    pub checks_noun: &'static str,
    pub no_assist_phantom: &'static str, // {} noun
    pub none_left: &'static str,         // {} noun
    pub nothing_undo: &'static str,
    pub nothing_redo: &'static str,
    pub coming: &'static str, // {} seconds
    pub recedes: &'static str,
    pub phantomed: &'static str, // {} lives
    pub won: &'static str,
    pub leave_confirm: &'static str,
    pub back_classic: &'static str,
    pub phantom_on: &'static str,
    pub haunted: &'static str,    // {} seconds
    pub phantom_in: &'static str, // {} seconds
    pub no_pack: &'static str,
    pub given_locked: &'static str,
    pub not_markable: &'static str,
    pub erase: &'static str,
    pub diff_hint: &'static str,
    pub classic: &'static str,
    pub rules_btn: &'static str,
    pub rules_classic: &'static str,
    pub rules_phantom: &'static str,
    pub close: &'static str,
}

pub const EN: T = T {
    pencil: "pencil",
    hint: "hint",
    check: "check",
    undo: "undo",
    redo: "redo",
    new_game: "new game",
    phantom: "phantom",
    easy: "easy",
    medium: "medium",
    hard: "hard",
    pen: "pen",
    pencil_mode: "pencil",
    loading: "loading engine…",
    engine_loading: "the engine is still loading…",
    solved: "solved — el abuelo would be proud.",
    all_good: "all good so far",
    wrong_cells: "{} wrong cell(s)",
    new_game_msg: "new {} game",
    hints_noun: "hints",
    checks_noun: "checks",
    no_assist_phantom: "no {} in phantom mode — trust your hand",
    none_left: "no {} left",
    nothing_undo: "nothing to undo",
    nothing_redo: "nothing to redo",
    coming: "the phantom is coming — place a correct number within {}s to hold it off",
    recedes: "the phantom recedes…",
    phantomed: "the sudoku phantomed — {} live(s) left",
    won: "the phantoms won — n for a new game",
    leave_confirm: "leave phantom mode? click phantom again to confirm",
    back_classic: "back to classic",
    phantom_on: "phantom mode — keep placing right, or the sudoku flips",
    haunted: "⚠ HAUNTED — {}s to place a number",
    phantom_in: "phantom in {}s",
    no_pack: "no asset pack could be loaded",
    given_locked: "that cell is a given",
    not_markable: "that cell already holds a value",
    erase: "erase",
    diff_hint: "leave phantom mode to change difficulty",
    classic: "classic",
    rules_btn: "rules",
    rules_classic: "Fill the grid so that every row, every column and every 3\u{d7}3 box holds the digits 1 to 9, each exactly once. The black digits are given \u{2014} they were always there. Tap a cell, then a digit from the palette, to write in red; the pencil leaves small marks while you think. You have 3 hints and 3 checks per game \u{2014} spend them wisely.",
    rules_phantom: "Phantom mode is a pact with the clock: keep placing correct digits, and the page stays yours. Stall too long and the haunting begins \u{2014} another grid fades in over the board. Place a correct digit before it settles to ward it off; fail, and the sudoku flips into its phantom: your correct work survives, but the puzzle reshapes around it. Three lives per game, no hints, no checks \u{2014} trust your hand.",
    close: "close",
};

pub const FR: T = T {
    pencil: "crayon",
    hint: "indice",
    check: "vérifier",
    undo: "annuler",
    redo: "rétablir",
    new_game: "nouvelle partie",
    phantom: "fantôme",
    easy: "facile",
    medium: "moyen",
    hard: "difficile",
    pen: "stylo",
    pencil_mode: "crayon",
    loading: "chargement du moteur…",
    engine_loading: "le moteur charge encore…",
    solved: "résolu — el abuelo serait fier.",
    all_good: "tout est bon pour l'instant",
    wrong_cells: "{} case(s) fausse(s)",
    new_game_msg: "nouvelle partie ({})",
    hints_noun: "indices",
    checks_noun: "vérifications",
    no_assist_phantom: "pas de {} en mode fantôme — fie-toi à ta main",
    none_left: "plus de {}",
    nothing_undo: "rien à annuler",
    nothing_redo: "rien à rétablir",
    coming: "le fantôme arrive — place un chiffre juste en {}s pour le repousser",
    recedes: "le fantôme recule…",
    phantomed: "le sudoku s'est fantômé — {} vie(s) restante(s)",
    won: "les fantômes ont gagné — n pour une nouvelle partie",
    leave_confirm: "quitter le mode fantôme ? clique encore pour confirmer",
    back_classic: "retour au classique",
    phantom_on: "mode fantôme — continue à jouer juste, ou le sudoku bascule",
    haunted: "⚠ HANTÉ — {}s pour placer un chiffre",
    phantom_in: "fantôme dans {}s",
    no_pack: "aucun pack d'images n'a pu être chargé",
    given_locked: "cette case est un chiffre donné",
    not_markable: "cette case contient déjà un chiffre",
    erase: "effacer",
    diff_hint: "quitte le mode fantôme pour changer de difficulté",
    classic: "classique",
    rules_btn: "r\u{e8}gles",
    rules_classic: "Remplis la grille pour que chaque ligne, chaque colonne et chaque carr\u{e9} 3\u{d7}3 contiennent les chiffres de 1 \u{e0} 9, une seule fois chacun. Les chiffres noirs sont donn\u{e9}s \u{2014} ils ont toujours \u{e9}t\u{e9} l\u{e0}. Touche une case puis un chiffre de la palette pour \u{e9}crire en rouge ; le crayon pose de petites notes pendant que tu r\u{e9}fl\u{e9}chis. Tu as 3 indices et 3 v\u{e9}rifications par partie \u{2014} d\u{e9}pense-les avec sagesse.",
    rules_phantom: "Le mode fant\u{f4}me est un pacte avec l'horloge : continue \u{e0} placer des chiffres justes, et la page reste \u{e0} toi. Tarde trop et la hantise commence \u{2014} une autre grille appara\u{ee}t en transparence sur la tienne. Place un chiffre juste avant qu'elle ne s'installe pour la repousser ; sinon, le sudoku bascule en son fant\u{f4}me : ton travail juste survit, mais l'\u{e9}nigme se recompose autour. Trois vies par partie, sans indices ni v\u{e9}rifications \u{2014} fie-toi \u{e0} ta main.",
    close: "fermer",
};

pub const ES: T = T {
    pencil: "lápiz",
    hint: "pista",
    check: "comprobar",
    undo: "deshacer",
    redo: "rehacer",
    new_game: "nueva partida",
    phantom: "fantasma",
    easy: "fácil",
    medium: "medio",
    hard: "difícil",
    pen: "bolígrafo",
    pencil_mode: "lápiz",
    loading: "cargando el motor…",
    engine_loading: "el motor sigue cargando…",
    solved: "resuelto — el abuelo estaría orgulloso.",
    all_good: "todo bien por ahora",
    wrong_cells: "{} celda(s) incorrecta(s)",
    new_game_msg: "nueva partida ({})",
    hints_noun: "pistas",
    checks_noun: "comprobaciones",
    no_assist_phantom: "sin {} en modo fantasma — confía en tu mano",
    none_left: "no quedan {}",
    nothing_undo: "nada que deshacer",
    nothing_redo: "nada que rehacer",
    coming: "el fantasma viene — coloca una cifra correcta en {}s para contenerlo",
    recedes: "el fantasma retrocede…",
    phantomed: "el sudoku se esfumó — {} vida(s) restante(s)",
    won: "los fantasmas ganaron — n para una nueva partida",
    leave_confirm: "¿salir del modo fantasma? haz clic de nuevo para confirmar",
    back_classic: "de vuelta al clásico",
    phantom_on: "modo fantasma — sigue acertando o el sudoku se transforma",
    haunted: "⚠ EMBRUJADO — {}s para colocar una cifra",
    phantom_in: "fantasma en {}s",
    no_pack: "no se pudo cargar ningún pack de imágenes",
    given_locked: "esa celda es una cifra dada",
    not_markable: "esa celda ya contiene una cifra",
    erase: "borrar",
    diff_hint: "sal del modo fantasma para cambiar la dificultad",
    classic: "clásico",
    rules_btn: "reglas",
    rules_classic: "Rellena la cuadr\u{ed}cula para que cada fila, cada columna y cada caja de 3\u{d7}3 contengan los d\u{ed}gitos del 1 al 9, una sola vez cada uno. Los d\u{ed}gitos negros vienen dados \u{2014} siempre estuvieron ah\u{ed}. Toca una celda y luego un d\u{ed}gito de la paleta para escribir en rojo; el l\u{e1}piz deja peque\u{f1}as notas mientras piensas. Tienes 3 pistas y 3 comprobaciones por partida \u{2014} g\u{e1}stalas con cabeza.",
    rules_phantom: "El modo fantasma es un pacto con el reloj: sigue colocando d\u{ed}gitos correctos y la p\u{e1}gina seguir\u{e1} siendo tuya. Si tardas demasiado, empieza el embrujo \u{2014} otra cuadr\u{ed}cula se trasluce sobre la tuya. Coloca un d\u{ed}gito correcto antes de que se asiente para ahuyentarla; si no, el sudoku se convierte en su fantasma: tu trabajo correcto sobrevive, pero el enigma se recompone a su alrededor. Tres vidas por partida, sin pistas ni comprobaciones \u{2014} conf\u{ed}a en tu mano.",
    close: "cerrar",
};

pub fn t(lang: Lang) -> &'static T {
    match lang {
        Lang::En => &EN,
        Lang::Fr => &FR,
        Lang::Es => &ES,
    }
}

pub fn fill(template: &str, value: &str) -> String {
    template.replacen("{}", value, 1)
}

pub fn diff_label(lang: Lang, key: &str) -> &'static str {
    let tr = t(lang);
    match key {
        "easy" => tr.easy,
        "hard" => tr.hard,
        _ => tr.medium,
    }
}

pub fn detect_lang() -> Lang {
    if let Ok(Some(store)) = window().local_storage() {
        if let Ok(Some(saved)) = store.get_item("lang") {
            match saved.as_str() {
                "fr" => return Lang::Fr,
                "es" => return Lang::Es,
                "en" => return Lang::En,
                _ => {}
            }
        }
    }
    let nav = window().navigator().language().unwrap_or_default();
    if nav.starts_with("fr") {
        Lang::Fr
    } else if nav.starts_with("es") {
        Lang::Es
    } else {
        Lang::En
    }
}

pub fn save_lang(lang: Lang) {
    if let Ok(Some(store)) = window().local_storage() {
        let _ = store.set_item(
            "lang",
            match lang {
                Lang::En => "en",
                Lang::Fr => "fr",
                Lang::Es => "es",
            },
        );
    }
}

/// The engine speaks English; translate the errors a player can actually cause.
pub fn translate_engine_error(lang: Lang, err: &str) -> String {
    match err {
        "cell is a given" => t(lang).given_locked.into(),
        "cell not markable" => t(lang).not_markable.into(),
        other => other.into(),
    }
}

