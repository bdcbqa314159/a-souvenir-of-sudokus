//! Every user-facing string, in the three languages, plus language
//! detection and persistence. "el abuelo" stays el abuelo everywhere.
use leptos::prelude::window;

/// A message as an EVENT, not as finished text: the words are produced at
/// render time from the language in force then, so switching language
/// retranslates what is already on screen (it used to freeze in whatever
/// language was current when the event happened).
#[derive(Clone, PartialEq)]
pub enum Msg {
    None,
    NoPack,
    EngineLoading,
    Solved(u32), // with the number of wrong digits placed on the way
    Checking,
    AllGood,
    WrongCells(usize),
    NewGame(String),
    NoAssist {
        hints: bool,
    },
    NoneLeft {
        hints: bool,
    },
    NothingUndo,
    NothingRedo,
    Coming(u64),
    Recedes,
    Phantomed(u32),
    Won,
    HangmanOn,
    HangmanDrew(u32),
    HangmanLost,
    LeaveConfirm,
    BackClassic,
    PhantomOn,
    /// a raw error string from the engine, translated when rendered
    Engine(String),
}

impl Msg {
    pub fn text(&self, lang: Lang) -> String {
        let tr = t(lang);
        match self {
            Msg::None => String::new(),
            Msg::NoPack => tr.no_pack.into(),
            Msg::EngineLoading => tr.engine_loading.into(),
            // a clean grid says so; otherwise the tally comes out at the end,
            // which is the only moment it is not an oracle
            Msg::Solved(0) => tr.solved.into(),
            Msg::Solved(n) => format!(
                "{} \u{00b7} {}",
                tr.solved,
                fill(tr.mistakes_tally, &n.to_string())
            ),
            // a verdict should be readable before the words are: the mark
            // carries it in any language
            // the words only: the wink and the three dots are put back by the
            // status line in main.rs, which animates them. The pun stays in
            // Spanish in every language — it is his, not a translation.
            Msg::Checking => "valvula check".into(),
            Msg::AllGood => format!("\u{2713} {}", tr.all_good),
            Msg::WrongCells(n) => format!("\u{2717} {}", fill(tr.wrong_cells, &n.to_string())),
            Msg::NewGame(d) => fill(tr.new_game_msg, diff_label(lang, d)),
            Msg::NoAssist { hints } => fill(
                tr.no_assist_phantom,
                if *hints {
                    tr.hints_noun
                } else {
                    tr.checks_noun
                },
            ),
            Msg::NoneLeft { hints } => fill(
                tr.none_left,
                if *hints {
                    tr.hints_noun
                } else {
                    tr.checks_noun
                },
            ),
            Msg::NothingUndo => tr.nothing_undo.into(),
            Msg::NothingRedo => tr.nothing_redo.into(),
            Msg::Coming(secs) => fill(tr.coming, &secs.to_string()),
            Msg::Recedes => tr.recedes.into(),
            Msg::Phantomed(lives) => fill(tr.phantomed, &lives.to_string()),
            Msg::Won => tr.won.into(),
            Msg::HangmanOn => tr.hangman_on.into(),
            Msg::HangmanDrew(left) => fill(tr.hangman_drew, &left.to_string()),
            Msg::HangmanLost => tr.hangman_lost.into(),
            Msg::LeaveConfirm => tr.leave_confirm.into(),
            Msg::BackClassic => tr.back_classic.into(),
            Msg::PhantomOn => tr.phantom_on.into(),
            Msg::Engine(e) => translate_engine_error(lang, e),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Classic,
    Phantom,
    /// help is always available, and every use draws one more stroke of the
    /// hangman — the sixth finishes him, and the game with him
    Hangman,
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
    pub wrong_cells: &'static str,  // {} count
    pub new_game_msg: &'static str, // {} difficulty label
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
    /// the pack toggle's other side: ordinary type, as against his hand. It
    /// used to borrow `classic`, which now names a mode — a button and a mode
    /// sharing a word was one word doing two jobs.
    pub regular: &'static str,
    pub rules_btn: &'static str,
    pub rules_classic: &'static str,
    pub rules_phantom: &'static str,
    pub close: &'static str,
    pub hangman: &'static str,
    pub hangman_ask: &'static str,
    pub hangman_on: &'static str,
    pub hangman_drew: &'static str,
    pub hangman_lost: &'static str,
    pub rules_hangman: &'static str,
    pub ctl_hangman: &'static str,
    pub solve: &'static str,
    pub history: &'static str,
    pub history_empty: &'static str,
    pub history_count: &'static str,
    pub history_clear: &'static str,
    pub ctl_history: &'static str,
    pub rules_controls: &'static str,
    pub ctl_digits: &'static str,
    pub ctl_erase: &'static str,
    pub ctl_pencil: &'static str,
    /// the aiming prompt; {} is the seconds left
    pub aim: &'static str,
    /// the end-of-grid tally; {} is how many wrong digits were placed
    pub mistakes_tally: &'static str,
    pub ctl_hint: &'static str,
    pub ctl_check: &'static str,
    pub ctl_undo: &'static str,
    pub ctl_new: &'static str,
    /// the three-way mode picker in the setup bar
    pub ctl_modes: &'static str,
    pub ctl_pack: &'static str,
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
    won: "the phantoms won — press N for a new game",
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
    regular: "regular",
    rules_btn: "rules",
    rules_classic: "Fill the grid so that every row, every column and every 3\u{d7}3 box holds the digits 1 to 9, each exactly once. The black digits are given \u{2014} they were always there. Tap a cell, then a digit from the palette, to write in red; the pencil leaves small marks while you think. You have 3 hints and 3 checks per game \u{2014} spend them wisely.",
    rules_phantom: "Phantom mode is a pact with the clock: keep placing correct digits, and the page stays yours. Stall too long and the haunting begins \u{2014} another grid fades in over the board. Place a correct digit before it settles to ward it off; fail, and the sudoku flips into its phantom: your correct work survives, but the puzzle reshapes around it. Three lives per game, no hints, no checks \u{2014} trust your hand.",
    close: "close",
    hangman: "hangman",
    hangman_ask: "every digit you place",
    hangman_on: "hangman mode \u{2014} he watches every digit you place, and draws himself when you are wrong",
    hangman_drew: "wrong \u{2014} he draws another stroke. {} left before he is finished",
    hangman_lost: "the hangman finished his drawing \u{2014} press N for a new game",
    rules_hangman: "No hints, no checks \u{2014} the hangman is your only answer, and he answers for free: every digit you place is interrogated at once. Right, and he stays exactly as he is. Wrong, and he draws one more stroke of himself \u{2014} first his gallows, then head, body, arms, legs. Ten mistakes finish the drawing, and the grid with it. You always know where you stand; the knowing is what costs.",
    ctl_hangman: "is interrogated the moment it lands: right and nothing happens, wrong and he draws.",
    solve: "solve",
    history: "solved",
    history_empty: "no finished grids yet \u{2014} the first one you solve is kept here.",
    history_count: "{} grids finished",
    history_clear: "forget them all",
    ctl_history: "the grids you have finished, newest first \u{2014} kept in this browser only.",
    rules_controls: "the buttons",
    ctl_digits: "tap a cell, then a digit \u{2014} it is written in his hand, in red.",
    ctl_erase: "rubs the chosen cell out: its digit, or its pencil marks.",
    ctl_pencil: "pencil mode \u{2014} digits land as small notes instead of an answer. Tap the same digit again to take a note back.",
    aim: "point at an empty square \u{2014} {}s, or he picks one",
    mistakes_tally: "{} wrong along the way",
    ctl_hint: "fills one correct cell for you. Point at an empty square and it lands there; take too long and it lands where it likes. Three per game, none in phantom or hangman mode.",
    ctl_check: "tells you how many of your digits are wrong \u{2014} not which ones. Three per game, none in phantom mode.",
    ctl_undo: "step back and forward through your own moves.",
    ctl_new: "deals a fresh grid \u{2014} easy, medium or hard, graded by the techniques it needs.",
    ctl_modes: "picks how you play: classic, the pact with the clock, or the hangman. Leaving the phantom\u{2019}s room asks twice \u{2014} the first press is the question.",
    ctl_pack: "switches between his handwriting and plain printed digits.",
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
    won: "les fantômes ont gagné — appuie sur N pour une nouvelle partie",
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
    regular: "r\u{e9}gulier",
    rules_btn: "r\u{e8}gles",
    rules_classic: "Remplis la grille pour que chaque ligne, chaque colonne et chaque carr\u{e9} 3\u{d7}3 contiennent les chiffres de 1 \u{e0} 9, une seule fois chacun. Les chiffres noirs sont donn\u{e9}s \u{2014} ils ont toujours \u{e9}t\u{e9} l\u{e0}. Touche une case puis un chiffre de la palette pour \u{e9}crire en rouge ; le crayon pose de petites notes pendant que tu r\u{e9}fl\u{e9}chis. Tu as 3 indices et 3 v\u{e9}rifications par partie \u{2014} d\u{e9}pense-les avec sagesse.",
    rules_phantom: "Le mode fant\u{f4}me est un pacte avec l'horloge : continue \u{e0} placer des chiffres justes, et la page reste \u{e0} toi. Tarde trop et la hantise commence \u{2014} une autre grille appara\u{ee}t en transparence sur la tienne. Place un chiffre juste avant qu'elle ne s'installe pour la repousser ; sinon, le sudoku bascule en son fant\u{f4}me : ton travail juste survit, mais l'\u{e9}nigme se recompose autour. Trois vies par partie, sans indices ni v\u{e9}rifications \u{2014} fie-toi \u{e0} ta main.",
    close: "fermer",
    hangman: "pendu",
    hangman_ask: "chaque chiffre que tu poses",
    hangman_on: "mode pendu \u{2014} il surveille chaque chiffre pos\u{e9}, et se dessine quand tu te trompes",
    hangman_drew: "faux \u{2014} il trace un trait de plus. Encore {} avant qu'il soit achev\u{e9}",
    hangman_lost: "le pendu a fini son dessin \u{2014} appuie sur N pour une nouvelle partie",
    rules_hangman: "Ni indices ni v\u{e9}rifications \u{2014} le pendu est ta seule r\u{e9}ponse, et il r\u{e9}pond gratuitement : chaque chiffre pos\u{e9} est interrog\u{e9} aussit\u{f4}t. Juste, il reste exactement tel quel. Faux, il trace un trait de plus \u{2014} d'abord sa potence, puis t\u{ea}te, corps, bras, jambes. Dix erreurs ach\u{e8}vent le dessin, et la grille avec. Tu sais toujours o\u{f9} tu en es ; c'est ce savoir qui co\u{fb}te.",
    ctl_hangman: "est interrog\u{e9} d\u{e8}s qu'il tombe : juste, rien ne bouge ; faux, il dessine.",
    solve: "r\u{e9}soudre",
    history: "r\u{e9}solues",
    history_empty: "aucune grille termin\u{e9}e pour l'instant \u{2014} la premi\u{e8}re que tu r\u{e9}sous sera gard\u{e9}e ici.",
    history_count: "{} grilles termin\u{e9}es",
    history_clear: "tout oublier",
    ctl_history: "les grilles que tu as termin\u{e9}es, les plus r\u{e9}centes d'abord \u{2014} gard\u{e9}es dans ce navigateur seulement.",
    rules_controls: "les boutons",
    ctl_digits: "touche une case, puis un chiffre \u{2014} il s'\u{e9}crit de sa main, en rouge.",
    ctl_erase: "efface la case choisie : son chiffre, ou ses notes au crayon.",
    ctl_pencil: "mode crayon \u{2014} les chiffres se posent en petites notes au lieu d'une r\u{e9}ponse. Retouche le m\u{ea}me chiffre pour retirer la note.",
    aim: "d\u{e9}signe une case vide \u{2014} {}s, sinon il choisit",
    mistakes_tally: "{} faux en chemin",
    ctl_hint: "remplit une case juste pour toi. D\u{e9}signe une case vide et l\u{2019}indice s\u{2019}y pose; trop lent et il se pose o\u{f9} il veut. Trois par partie, aucune en mode fant\u{f4}me ou pendu.",
    ctl_check: "te dit combien de tes chiffres sont faux \u{2014} pas lesquels. Trois par partie, aucune en mode fant\u{f4}me.",
    ctl_undo: "reviens en arri\u{e8}re et en avant dans tes propres coups.",
    ctl_new: "distribue une nouvelle grille \u{2014} facile, moyen ou difficile, selon les techniques qu'elle exige.",
    ctl_modes: "choisit comment tu joues : classique, le pacte avec l\u{2019}horloge, ou le pendu. Quitter la chambre du fant\u{f4}me demande deux fois \u{2014} la premi\u{e8}re pression est la question.",
    ctl_pack: "bascule entre son \u{e9}criture et des chiffres imprim\u{e9}s.",
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
    won: "los fantasmas ganaron — pulsa N para una nueva partida",
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
    regular: "regular",
    rules_btn: "reglas",
    rules_classic: "Rellena la cuadr\u{ed}cula para que cada fila, cada columna y cada caja de 3\u{d7}3 contengan los d\u{ed}gitos del 1 al 9, una sola vez cada uno. Los d\u{ed}gitos negros vienen dados \u{2014} siempre estuvieron ah\u{ed}. Toca una celda y luego un d\u{ed}gito de la paleta para escribir en rojo; el l\u{e1}piz deja peque\u{f1}as notas mientras piensas. Tienes 3 pistas y 3 comprobaciones por partida \u{2014} g\u{e1}stalas con cabeza.",
    rules_phantom: "El modo fantasma es un pacto con el reloj: sigue colocando d\u{ed}gitos correctos y la p\u{e1}gina seguir\u{e1} siendo tuya. Si tardas demasiado, empieza el embrujo \u{2014} otra cuadr\u{ed}cula se trasluce sobre la tuya. Coloca un d\u{ed}gito correcto antes de que se asiente para ahuyentarla; si no, el sudoku se convierte en su fantasma: tu trabajo correcto sobrevive, pero el enigma se recompone a su alrededor. Tres vidas por partida, sin pistas ni comprobaciones \u{2014} conf\u{ed}a en tu mano.",
    close: "cerrar",
    hangman: "ahorcado",
    hangman_ask: "cada d\u{ed}gito que colocas",
    hangman_on: "modo ahorcado \u{2014} vigila cada d\u{ed}gito que colocas, y se dibuja cuando te equivocas",
    hangman_drew: "mal \u{2014} traza un trazo m\u{e1}s. Quedan {} antes de que est\u{e9} terminado",
    hangman_lost: "el ahorcado termin\u{f3} su dibujo \u{2014} pulsa N para una nueva partida",
    rules_hangman: "Sin pistas ni comprobaciones \u{2014} el ahorcado es tu \u{fa}nica respuesta, y responde gratis: cada d\u{ed}gito que colocas se interroga al instante. Si est\u{e1} bien, \u{e9}l sigue igual. Si est\u{e1} mal, traza un trazo m\u{e1}s de s\u{ed} mismo \u{2014} primero su horca, luego cabeza, cuerpo, brazos, piernas. Diez errores terminan el dibujo, y la cuadr\u{ed}cula con \u{e9}l. Siempre sabes d\u{f3}nde est\u{e1}s; saberlo es lo que cuesta.",
    ctl_hangman: "se interroga en cuanto cae: bien y nada pasa, mal y \u{e9}l dibuja.",
    solve: "resolver",
    history: "resueltas",
    history_empty: "a\u{fa}n no hay cuadr\u{ed}culas terminadas \u{2014} la primera que resuelvas se guardar\u{e1} aqu\u{ed}.",
    history_count: "{} cuadr\u{ed}culas terminadas",
    history_clear: "olvidarlas todas",
    ctl_history: "las cuadr\u{ed}culas que has terminado, las m\u{e1}s recientes primero \u{2014} guardadas solo en este navegador.",
    rules_controls: "los botones",
    ctl_digits: "toca una celda y luego un d\u{ed}gito \u{2014} se escribe de su pu\u{f1}o y letra, en rojo.",
    ctl_erase: "borra la celda elegida: su d\u{ed}gito o sus notas a l\u{e1}piz.",
    ctl_pencil: "modo l\u{e1}piz \u{2014} los d\u{ed}gitos se anotan en peque\u{f1}o en vez de ser una respuesta. Toca el mismo d\u{ed}gito otra vez para quitar la nota.",
    aim: "se\u{f1}ala una casilla vac\u{ed}a \u{2014} {}s, o la elige \u{e9}l",
    mistakes_tally: "{} errados por el camino",
    ctl_hint: "rellena una celda correcta por ti. Se\u{f1}ala una casilla vac\u{ed}a y la pista cae ah\u{ed}; si tardas, cae donde quiera. Tres por partida, ninguna en modo fantasma o del ahorcado.",
    ctl_check: "te dice cu\u{e1}ntos de tus d\u{ed}gitos est\u{e1}n mal \u{2014} no cu\u{e1}les. Tres por partida, ninguna en modo fantasma.",
    ctl_undo: "retrocede y avanza por tus propias jugadas.",
    ctl_new: "reparte una cuadr\u{ed}cula nueva \u{2014} f\u{e1}cil, medio o dif\u{ed}cil, seg\u{fa}n las t\u{e9}cnicas que exige.",
    ctl_modes: "elige c\u{f3}mo juegas: cl\u{e1}sico, el pacto con el reloj, o el ahorcado. Salir del cuarto del fantasma pregunta dos veces \u{2014} la primera pulsaci\u{f3}n es la pregunta.",
    ctl_pack: "alterna entre su escritura y d\u{ed}gitos impresos.",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_is_wired() {
        assert_eq!(t(Lang::En).hint, EN.hint);
        assert_eq!(t(Lang::Fr).hint, FR.hint);
        assert_eq!(t(Lang::Es).hint, ES.hint);
        for tr in [&EN, &FR, &ES] {
            assert!(!tr.rules_classic.is_empty() && !tr.rules_phantom.is_empty());
            assert!(
                tr.wrong_cells.contains("{}"),
                "placeholder must survive translation"
            );
        }
    }

    #[test]
    fn fill_substitutes_once() {
        assert_eq!(fill("{} wrong cell(s)", "3"), "3 wrong cell(s)");
        assert_eq!(fill("no placeholder", "3"), "no placeholder");
    }

    #[test]
    fn difficulty_labels_translate_and_fall_back() {
        assert_eq!(diff_label(Lang::Fr, "easy"), FR.easy);
        assert_eq!(diff_label(Lang::Es, "hard"), ES.hard);
        assert_eq!(
            diff_label(Lang::En, "nonsense"),
            EN.medium,
            "unknown label falls back"
        );
    }

    #[test]
    fn engine_errors_translate_without_swallowing_unknowns() {
        let known = translate_engine_error(Lang::Fr, "cell is a given");
        assert_eq!(known, FR.given_locked);
        let unknown = translate_engine_error(Lang::Fr, "something new from the engine");
        assert!(
            unknown.contains("something new"),
            "unknown errors must still reach the player"
        );
    }
}
