use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    routing::get,
    Router,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use if_addrs::{get_if_addrs, IfAddr};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use serde::{Deserialize, Serialize};
use slint::ComponentHandle;
use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

slint::slint! {
    import { VerticalBox, HorizontalBox } from "std-widgets.slint";

    export component MainWindow inherits Window {
        in property <string> phase: "lobby";
        in property <int> timer: 0;
        in property <string> prompt: "";
        in property <string> lobby_url: "partybox.play";
        in property <[string]> player_names: [];
        in property <[string]> ready_names: [];
        in property <[string]> result_lines: [];
        in property <int> round: 1;

        background: #1a1a2e;
        width: 1024px;
        height: 600px;

        VerticalBox {
            padding: 40px;
            alignment: start;

            HorizontalBox {
                Text {
                    text: "PartyStation";
                    font-size: 60px;
                    font-weight: 900;
                    color: #f1c40f;
                }
                if phase != "lobby" && phase != "results" : Text {
                    text: "  Round " + round + " / 3";
                    font-size: 30px;
                    color: #888;
                    vertical-alignment: center;
                }
            }

            if phase == "lobby" : VerticalBox {
                spacing: 15px;
                Text {
                    text: "1. Join 'PartyBox' Wi-Fi";
                    font-size: 32px;
                    color: white;
                    horizontal-alignment: center;
                }
                Text {
                    text: "2. Go to " + lobby_url;
                    font-size: 32px;
                    color: white;
                    horizontal-alignment: center;
                }
                for name in player_names : Text {
                    text: "👤 " + name;
                    font-size: 32px;
                    color: #f1c40f;
                    horizontal-alignment: center;
                }
            }

            if phase == "prompting" : VerticalBox {
                alignment: center;
                spacing: 30px;
                Text { text: "GET READY TO ANSWER!"; font-size: 40px; color: #e94560; horizontal-alignment: center; }
                Text { text: timer + "s"; font-size: 100px; font-weight: 900; color: #f1c40f; horizontal-alignment: center; }

                HorizontalBox {
                    alignment: center;
                    spacing: 20px;
                    for name in ready_names : Rectangle {
                        background: rgb(46, 204, 113);
                        border-radius: 10px;
                        height: 60px;
                        width: 150px;
                        Text { text: name; font-size: 20px; font-weight: 800; color: white; }
                    }
                }
            }

            if phase == "voting" || phase == "reveal" : VerticalBox {
                spacing: 40px;
                Rectangle {
                    background: rgb(22, 33, 62);
                    border-radius: 30px;
                    height: 200px;
                    Text { text: prompt; font-size: 50px; color: white; horizontal-alignment: center; vertical-alignment: center; wrap: word-wrap; }
                }
                if phase == "voting" : Text { text: "VOTE NOW! " + timer + "s"; font-size: 45px; color: #e94560; horizontal-alignment: center; }
            }

            if phase == "results" : VerticalBox {
                spacing: 20px;
                Text { text: "THE CHAMPIONS"; font-size: 60px; color: #f1c40f; horizontal-alignment: center; }
                for line in result_lines : Text {
                    text: line;
                    font-size: 40px;
                    color: white;
                    horizontal-alignment: center;
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Player {
    id: usize,
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Question {
    prompt: String,
    player_ids: (usize, usize),
    answers: HashMap<usize, String>,
    votes: HashMap<usize, usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ClientMessage {
    Join {
        name: String,
    },
    StartGame,
    #[serde(rename_all = "camelCase")]
    SubmitAnswer {
        question_index: usize,
        answer: String,
    },
    #[serde(rename_all = "camelCase")]
    SubmitVote {
        question_index: usize,
        target_id: usize,
    },
    ResetToLobby,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
enum GamePhase {
    Lobby,
    Prompting,
    Voting,
    Reveal,
    Results,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ServerMessage {
    #[serde(rename_all = "camelCase")]
    Welcome {
        id: usize,
        phase: GamePhase,
        players: Vec<Player>,
        scores: HashMap<usize, i32>,
        my_questions: Vec<(usize, String)>,
        current_question: Option<Question>,
        current_question_index: usize,
        questions: Vec<Question>,
        can_start: bool,
        timer: u32,
        round: u32,
    },
    #[serde(rename_all = "camelCase")]
    LobbyState {
        phase: GamePhase,
        players: Vec<Player>,
        can_start: bool,
    },
    #[serde(rename_all = "camelCase")]
    GameState {
        phase: GamePhase,
        scores: HashMap<usize, i32>,
        my_questions: Option<Vec<(usize, String)>>,
        current_question: Option<Question>,
        current_question_index: Option<usize>,
        questions: Option<Vec<Question>>,
        timer: u32,
        round: u32,
        players: Option<Vec<Player>>,
    },
    TimerTick {
        timer: u32,
    },
}

struct AppState {
    players: HashMap<usize, Player>,
    scores: HashMap<usize, i32>,
    questions: Vec<Question>,
    current_question_index: usize,
    phase: GamePhase,
    timer: u32,
    round: u32,
    next_id: usize,
    tx: broadcast::Sender<ServerMessage>,
}

fn generate_questions(players: &[usize], _round: u32) -> Vec<Question> {
    let mut prompts = vec![
        "A new law states that everyone must __________ once a day.",
        "If I were a superhero, my useless power would be __________.",
        "The best way to spice up a boring wedding is __________.",
        "The title of my autobiography would be: 'The Man, The Myth, The __________.'",
        "I don't need a therapist, I just need __________.",
    ];
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    prompts.shuffle(&mut rng);
    let n = players.len();
    let mut qs = Vec::new();
    for i in 0..n {
        qs.push(Question {
            prompt: prompts[i % prompts.len()].to_string(),
            player_ids: (players[i], players[(i + 1) % n]),
            answers: HashMap::new(),
            votes: HashMap::new(),
        });
    }
    qs
}

fn ensure_slint_backend() {
    if std::env::var("SLINT_BACKEND").is_err() {
        if cfg!(target_os = "linux") {
            std::env::set_var("SLINT_BACKEND", "linuxkms-noseat");
        } else {
            std::env::set_var("SLINT_BACKEND", "winit");
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    ensure_slint_backend();
    let addr = SocketAddr::from(([0, 0, 0, 0], 80));
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    let my_ip = get_if_addrs()?
        .into_iter()
        .find(|iface| !iface.is_loopback() && matches!(iface.addr, IfAddr::V4(_)))
        .map(|iface| iface.addr.ip())
        .unwrap_or(IpAddr::V4("127.0.0.1".parse().unwrap()));
    let properties = [("path", "/")];
    let service_info = ServiceInfo::new(
        "_http._tcp.local.",
        "partystation",
        "partystation.local.",
        my_ip.to_string(),
        80,
        &properties[..],
    )
    .unwrap();
    mdns.register(service_info)
        .expect("Failed to register service");

    let (tx, _rx) = broadcast::channel(100);
    let state = Arc::new(RwLock::new(AppState {
        players: HashMap::new(),
        scores: HashMap::new(),
        questions: Vec::new(),
        current_question_index: 0,
        phase: GamePhase::Lobby,
        timer: 0,
        round: 1,
        next_id: 1,
        tx: tx.clone(),
    }));

    let state_axum = state.clone();
    tokio::spawn(async move {
        let app = Router::new()
            .route("/ws", get(ws_handler))
            .nest_service("/", ServeDir::new("public"))
            .with_state(state_axum);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    let state_timer = state.clone();
    let tx_timer = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let (should_send_full, res) = {
                let mut s = state_timer.write().unwrap();
                if s.phase != GamePhase::Lobby && s.timer > 0 {
                    s.timer -= 1;
                    let mut phase_changed = false;
                    if s.timer == 0 {
                        match s.phase {
                            GamePhase::Prompting => {
                                s.phase = GamePhase::Voting;
                                s.current_question_index = 0;
                                s.timer = 10;
                                phase_changed = true;
                            }
                            GamePhase::Voting => {
                                s.phase = GamePhase::Reveal;
                                s.timer = 5;
                                phase_changed = true;
                            }
                            GamePhase::Reveal => {
                                s.current_question_index += 1;
                                if s.current_question_index >= s.questions.len() {
                                    if s.round < 3 {
                                        s.round += 1;
                                        s.phase = GamePhase::Prompting;
                                        s.timer = 60;
                                        let p_ids: Vec<_> = s.players.keys().cloned().collect();
                                        s.questions = generate_questions(&p_ids, s.round);
                                        s.current_question_index = 0;
                                    } else {
                                        s.phase = GamePhase::Results;
                                    }
                                } else {
                                    s.phase = GamePhase::Voting;
                                    s.timer = 10;
                                }
                                phase_changed = true;
                            }
                            _ => {}
                        }
                    }
                    if phase_changed {
                        (
                            true,
                            Some((
                                s.phase.clone(),
                                s.scores.clone(),
                                s.questions.get(s.current_question_index).cloned(),
                                s.current_question_index,
                                s.questions.clone(),
                                s.timer,
                                s.round,
                                s.players.values().cloned().collect(),
                            )),
                        )
                    } else {
                        let _ = s.tx.send(ServerMessage::TimerTick { timer: s.timer });
                        (false, None)
                    }
                } else {
                    (false, None)
                }
            };
            if should_send_full {
                if let Some((
                    phase,
                    scores,
                    current_question,
                    current_question_index,
                    questions,
                    timer,
                    round,
                    players,
                )) = res
                {
                    let _ = tx_timer.send(ServerMessage::GameState {
                        phase,
                        scores,
                        my_questions: None,
                        current_question,
                        current_question_index: Some(current_question_index),
                        questions: Some(questions),
                        timer,
                        round,
                        players: Some(players),
                    });
                }
            }
        }
    });

    let window = MainWindow::new().unwrap();
    let window_handle = window.as_weak();
    let mut rx = tx.subscribe();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            while let Ok(msg) = rx.recv().await {
                let w_handle = window_handle.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = w_handle.upgrade() {
                        match msg {
                            ServerMessage::Welcome {
                                phase,
                                players,
                                timer,
                                current_question,
                                round,
                                ..
                            } => {
                                let p_str: String = serde_json::to_value(&phase)
                                    .unwrap()
                                    .as_str()
                                    .unwrap()
                                    .into();
                                w.set_phase(slint::SharedString::from(p_str));
                                w.set_timer(timer as i32);
                                w.set_round(round as i32);
                                if let Some(q) = current_question {
                                    w.set_prompt(slint::SharedString::from(q.prompt));
                                }
                                let names: Vec<slint::SharedString> = players
                                    .iter()
                                    .map(|p| slint::SharedString::from(p.name.clone()))
                                    .collect();
                                w.set_player_names(
                                    std::rc::Rc::new(slint::VecModel::from(names)).into(),
                                );
                            }
                            ServerMessage::GameState {
                                phase,
                                timer,
                                current_question,
                                round,
                                questions,
                                players,
                                scores,
                                ..
                            } => {
                                let p_str: String = serde_json::to_value(&phase)
                                    .unwrap()
                                    .as_str()
                                    .unwrap()
                                    .into();
                                w.set_phase(slint::SharedString::from(p_str));
                                w.set_timer(timer as i32);
                                w.set_round(round as i32);
                                if let Some(q) = current_question {
                                    w.set_prompt(slint::SharedString::from(q.prompt));
                                }
                                if let (Some(qs), Some(ps)) = (questions, players) {
                                    let ready: Vec<slint::SharedString> = ps
                                        .iter()
                                        .filter(|p| {
                                            qs.iter()
                                                .filter(|q| {
                                                    q.player_ids.0 == p.id || q.player_ids.1 == p.id
                                                })
                                                .all(|q| q.answers.contains_key(&p.id))
                                        })
                                        .map(|p| slint::SharedString::from(p.name.clone()))
                                        .collect();
                                    w.set_ready_names(
                                        std::rc::Rc::new(slint::VecModel::from(ready)).into(),
                                    );
                                    if phase == GamePhase::Results {
                                        let mut sorted = ps.clone();
                                        sorted.sort_by(|a, b| {
                                            scores
                                                .get(&b.id)
                                                .unwrap_or(&0)
                                                .cmp(scores.get(&a.id).unwrap_or(&0))
                                        });
                                        let lines: Vec<slint::SharedString> = sorted
                                            .iter()
                                            .map(|p| {
                                                slint::SharedString::from(format!(
                                                    "{}: {} pts",
                                                    p.name,
                                                    scores.get(&p.id).unwrap_or(&0)
                                                ))
                                            })
                                            .collect();
                                        w.set_result_lines(
                                            std::rc::Rc::new(slint::VecModel::from(lines)).into(),
                                        );
                                    }
                                }
                            }
                            ServerMessage::LobbyState { phase, players, .. } => {
                                let p_str: String = serde_json::to_value(&phase)
                                    .unwrap()
                                    .as_str()
                                    .unwrap()
                                    .into();
                                w.set_phase(slint::SharedString::from(p_str));
                                let names: Vec<slint::SharedString> = players
                                    .iter()
                                    .map(|p| slint::SharedString::from(p.name.clone()))
                                    .collect();
                                w.set_player_names(
                                    std::rc::Rc::new(slint::VecModel::from(names)).into(),
                                );
                            }
                            ServerMessage::TimerTick { timer } => {
                                w.set_timer(timer as i32);
                            }
                        }
                    }
                });
            }
        });
    });

    window.run().unwrap();
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RwLock<AppState>>>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<RwLock<AppState>>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.read().unwrap().tx.subscribe();
    let mut my_id: Option<usize> = None;
    loop {
        tokio::select! {
            Ok(msg) = rx.recv() => {
                let json = serde_json::to_string(&msg).unwrap();
                if sender.send(Message::Text(json.into())).await.is_err() { break; }
            }
            Some(Ok(msg)) = receiver.next() => {
                if let Message::Text(text) = msg {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        match client_msg {
                            ClientMessage::Join { name } => {
                                let (id, players, phase, scores, my_questions, current_question, current_question_index, questions, can_start, timer, round) = {
                                    let mut s = state.write().unwrap();
                                    let id = s.next_id; s.next_id += 1; my_id = Some(id);
                                    let player = Player { id, name: name.clone() };
                                    s.players.insert(id, player);
                                    let mut players = s.players.values().cloned().collect::<Vec<_>>();
                                    players.sort_by_key(|p| p.id);
                                    let my_questions = s.questions.iter().enumerate().filter(|(_, q)| q.player_ids.0 == id || q.player_ids.1 == id).map(|(i, q)| (i, q.prompt.clone())).collect();
                                    let current_question = s.questions.get(s.current_question_index).cloned();
                                    (id, players, s.phase.clone(), s.scores.clone(), my_questions, current_question, s.current_question_index, s.questions.clone(), s.players.len() >= 3, s.timer, s.round)
                                };
                                let _ = sender.send(Message::Text(serde_json::to_string(&ServerMessage::Welcome { id, phase, players: players.clone(), scores, my_questions, current_question, current_question_index, questions, can_start, timer, round }).unwrap().into())).await;
                                let _ = state.read().unwrap().tx.send(ServerMessage::LobbyState { phase: state.read().unwrap().phase.clone(), players, can_start: state.read().unwrap().players.len() >= 3 });
                            }
                            ClientMessage::StartGame => {
                                let (tx, questions, timer, round, players) = {
                                    let mut s = state.write().unwrap();
                                    if s.players.len() < 3 { drop(s); continue; }
                                    let p_ids: Vec<_> = s.players.keys().cloned().collect();
                                    s.phase = GamePhase::Prompting; s.timer = 60; s.round = 1;
                                    s.scores.clear(); for id in &p_ids { s.scores.insert(*id, 0); }
                                    s.questions = generate_questions(&p_ids, s.round);
                                    (s.tx.clone(), s.questions.clone(), s.timer, s.round, s.players.values().cloned().collect::<Vec<_>>())
                                };
                                let _ = tx.send(ServerMessage::GameState { phase: GamePhase::Prompting, scores: state.read().unwrap().scores.clone(), my_questions: None, current_question: None, current_question_index: Some(0), questions: Some(questions), timer, round, players: Some(players) });
                            }
                            ClientMessage::SubmitAnswer { question_index, answer } => {
                                let res = {
                                    let mut s = state.write().unwrap();
                                    if s.phase == GamePhase::Prompting {
                                        if let Some(q) = s.questions.get_mut(question_index) { if let Some(id) = my_id { q.answers.insert(id, answer); } }
                                        if s.questions.iter().all(|q| q.answers.len() == 2) { s.phase = GamePhase::Voting; s.current_question_index = 0; s.timer = 10; }
                                        Some((s.tx.clone(), s.phase.clone(), s.scores.clone(), s.questions.get(s.current_question_index).cloned(), s.current_question_index, s.questions.clone(), s.timer, s.round, s.players.values().cloned().collect()))
                                    } else { None }
                                };
                                if let Some((tx, phase, scores, current_question, current_question_index, questions, timer, round, players)) = res {
                                    let _ = tx.send(ServerMessage::GameState { phase: phase.clone(), scores, my_questions: None, current_question: if phase != GamePhase::Prompting { current_question } else { None }, current_question_index: Some(current_question_index), questions: Some(questions), timer, round, players: Some(players) });
                                }
                            }
                            ClientMessage::SubmitVote { question_index, target_id } => {
                                let res = {
                                    let mut s = state.write().unwrap();
                                    let num_players = s.players.len();
                                    if s.phase == GamePhase::Voting && s.current_question_index == question_index {
                                        let mut should_reveal = false; let mut winners = Vec::new();
                                        if let Some(q) = s.questions.get_mut(question_index) {
                                            if let Some(id) = my_id { if q.player_ids.0 != id && q.player_ids.1 != id { q.votes.insert(id, target_id); } }
                                            if q.votes.len() == num_players - 2 { should_reveal = true; winners = q.votes.values().cloned().collect(); }
                                        }
                                        if should_reveal { s.phase = GamePhase::Reveal; s.timer = 5; let multiplier = s.round; for wid in winners { *s.scores.entry(wid).or_insert(0) += 100 * multiplier as i32; } }
                                        Some((s.tx.clone(), s.phase.clone(), s.scores.clone(), s.questions.get(s.current_question_index).cloned(), s.current_question_index, s.questions.clone(), s.timer, s.round, s.players.values().cloned().collect()))
                                    } else { None }
                                };
                                if let Some((tx, phase, scores, current_question, current_question_index, questions, timer, round, players)) = res {
                                    let _ = tx.send(ServerMessage::GameState { phase, scores, my_questions: None, current_question, current_question_index: Some(current_question_index), questions: Some(questions), timer, round, players: Some(players) });
                                }
                            }
                            ClientMessage::ResetToLobby => {
                                let (tx, players, phase) = {
                                    let mut s = state.write().unwrap();
                                    s.phase = GamePhase::Lobby; s.scores.clear(); s.questions.clear(); s.current_question_index = 0; s.round = 1;
                                    let tx = s.tx.clone(); let mut players = s.players.values().cloned().collect::<Vec<_>>();
                                    players.sort_by_key(|p| p.id); (tx, players, s.phase.clone())
                                };
                                let _ = tx.send(ServerMessage::LobbyState { phase, players: players.clone(), can_start: players.len() >= 3 });
                            }
                        }
                    }
                }
            }
            else => break,
        }
    }
}
