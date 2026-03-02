const socket = new WebSocket(`ws://${window.location.host}/ws`);

const joinScreen = document.getElementById('join-screen');
const lobbyScreen = document.getElementById('lobby-screen');
const nameInput = document.getElementById('name-input');
const joinButton = document.getElementById('join-button');
const playerList = document.getElementById('player-list');
const startButton = document.getElementById('start-button');
const gameArea = document.getElementById('game-area');

let myName = '';
let myId = null;
let currentPhase = 'lobby';
let myQuestions = [];
let allQuestions = [];
let currentQuestionIndex = 0;
let playerMap = {};
let canStartGame = false;

joinButton.addEventListener('click', () => {
    const name = nameInput.value.trim();
    if (name) {
        myName = name;
        socket.send(JSON.stringify({ type: 'join', name: name }));
    }
});

startButton.addEventListener('click', () => {
    if (canStartGame) {
        socket.send(JSON.stringify({ type: 'startGame' }));
    }
});

socket.addEventListener('message', (event) => {
    const msg = JSON.parse(event.data);

    if (msg.type === 'timerTick') {
        updateTimerDisplay(msg.timer);
        return;
    }

    if (msg.timer !== undefined) {
        updateTimerDisplay(msg.timer);
    }

    switch (msg.type) {
        case 'welcome':
            myId = msg.id;
            currentPhase = msg.phase;
            allQuestions = msg.questions;
            currentQuestionIndex = msg.currentQuestionIndex !== undefined ? msg.currentQuestionIndex : msg.current_question_index;
            canStartGame = msg.canStart !== undefined ? msg.canStart : msg.can_start;
            refreshMyQuestions();
            updateUI(msg.players);
            if (currentPhase !== 'lobby') {
                updateGame(msg.phase, msg.scores, msg.currentQuestion !== undefined ? msg.currentQuestion : msg.current_question, currentQuestionIndex);
            }
            break;
        case 'lobbyState':
            currentPhase = msg.phase;
            canStartGame = msg.canStart !== undefined ? msg.canStart : msg.can_start;
            updateUI(msg.players);
            break;
        case 'gameState':
            currentPhase = msg.phase;
            if (msg.questions) {
                allQuestions = msg.questions;
                refreshMyQuestions();
            }
            const qIdx = msg.currentQuestionIndex !== undefined ? msg.currentQuestionIndex : msg.current_question_index;
            if (qIdx !== undefined) currentQuestionIndex = qIdx;
            
            lobbyScreen.style.display = currentPhase === 'lobby' ? 'block' : 'none';
            gameArea.style.display = currentPhase !== 'lobby' ? 'block' : 'none';
            updateGame(msg.phase, msg.scores, msg.currentQuestion !== undefined ? msg.currentQuestion : msg.current_question, currentQuestionIndex);
            break;
    }
});

function updateTimerDisplay(seconds) {
    let timerEl = document.getElementById('timer-banner');
    if (!timerEl) {
        timerEl = document.createElement('div');
        timerEl.id = 'timer-banner';
        timerEl.style = "position: fixed; top: 0; left: 0; width: 100%; background: var(--accent-1); color: white; font-weight: bold; padding: 10px; z-index: 1000; text-align: center; font-size: 1.2rem; box-shadow: 0 2px 5px rgba(0,0,0,0.3);";
        document.body.prepend(timerEl);
    }
    if (seconds > 0 && currentPhase !== 'lobby') {
        timerEl.innerText = `⏳ ${seconds}s Remaining`;
        timerEl.style.display = 'block';
    } else {
        timerEl.style.display = 'none';
    }
}

function refreshMyQuestions() {
    myQuestions = allQuestions.map((q, idx) => ({ q, idx }))
        .filter(item => {
            const pIds = item.q.playerIds !== undefined ? item.q.playerIds : item.q.player_ids;
            return pIds[0] === myId || pIds[1] === myId;
        })
        .map(item => [item.idx, item.q.prompt]);
}

function updateUI(players) {
    if (myId === null) return;
    joinScreen.style.display = 'none';
    playerMap = {};
    players.forEach(p => playerMap[p.id] = p.name);

    if (currentPhase === 'lobby') {
        lobbyScreen.style.display = 'block';
        gameArea.style.display = 'none';
        gameArea.innerHTML = '';
        gameArea.removeAttribute('data-current-q-idx');
    }

    playerList.innerHTML = '';
    let hostId = players.length > 0 ? players[0].id : null;
    players.forEach((player) => {
        const li = document.createElement('li');
        li.textContent = (player.id === myId ? '⭐️ ' : '') + player.name + (player.id === hostId ? ' (Host)' : '');
        playerList.appendChild(li);
    });

    const isHost = hostId === myId;
    startButton.style.display = isHost && currentPhase === 'lobby' ? 'block' : 'none';
    startButton.disabled = !canStartGame;
    startButton.title = canStartGame ? "" : "Need 3 players to start";
}

function updateGame(phase, scores, currentQuestion, qIndex) {
    if (phase === 'prompting') {
        const unanswered = myQuestions.filter(([idx, _]) => {
            const qData = allQuestions[idx];
            return !(qData && qData.answers[myId]);
        });

        if (unanswered.length === 0) {
            gameArea.innerHTML = '<h2>All answered!</h2><p>Watch the TV...</p>';
            gameArea.removeAttribute('data-current-q-idx');
            return;
        }

        const [idx, prompt] = unanswered[0];
        if (gameArea.getAttribute('data-current-q-idx') === idx.toString()) return; 
        
        gameArea.setAttribute('data-current-q-idx', idx);
        gameArea.innerHTML = `
            <h2>Question ${myQuestions.length - unanswered.length + 1} of ${myQuestions.length}</h2>
            <div class="prompt-card">
                <p class="prompt-text">${prompt}</p>
                <input type="text" id="answer-input" placeholder="">
                <button id="submit-btn">Submit</button>
            </div>
        `;
        
        document.getElementById('submit-btn').onclick = () => {
            const val = document.getElementById('answer-input').value.trim();
            if (val) {
                socket.send(JSON.stringify({ type: 'submitAnswer', questionIndex: idx, answer: val }));
                gameArea.innerHTML = '<h2>Sending...</h2>';
                gameArea.removeAttribute('data-current-q-idx');
            }
        };
    } else {
        gameArea.removeAttribute('data-current-q-idx');
        if (phase === 'voting') {
            if (!currentQuestion) {
                gameArea.innerHTML = '<h2>Loading battle...</h2>';
                return;
            }
            const pIds = currentQuestion.playerIds !== undefined ? currentQuestion.playerIds : currentQuestion.player_ids;
            const answeredByMe = pIds[0] === myId || pIds[1] === myId;
            gameArea.innerHTML = `<h2>Vote!</h2><p class="prompt-text">${currentQuestion.prompt}</p>`;
            
            if (answeredByMe) {
                gameArea.innerHTML += '<p>You are in this battle! Watch the TV.</p>';
            } else {
                const hasVoted = currentQuestion.votes[myId];
                if (hasVoted) {
                     gameArea.innerHTML += '<p>Vote cast! Waiting for results...</p>';
                } else {
                    const options = document.createElement('div');
                    for (const [id, answer] of Object.entries(currentQuestion.answers)) {
                        const btn = document.createElement('button');
                        btn.className = 'vote-button';
                        btn.textContent = answer;
                        btn.onclick = () => {
                            socket.send(JSON.stringify({ type: 'submitVote', questionIndex: qIndex, targetId: parseInt(id) }));
                        };
                        options.appendChild(btn);
                    }
                    gameArea.appendChild(options);
                }
            }
        } else if (phase === 'reveal') {
            gameArea.innerHTML = '<h2>Revealing results...</h2><p>Look at the TV!</p>';
        } else if (phase === 'results') {
            gameArea.innerHTML = '<h2>Final Standings</h2><div id="score-display"></div>';
            let scoreStr = '<ul>';
            const sortedScores = Object.entries(scores).sort((a, b) => b[1] - a[1]);
            for (const [id, score] of sortedScores) {
                scoreStr += `<li>${playerMap[id] || id}: ${score} pts</li>`;
            }
            scoreStr += '</ul><button id="lobby-btn">Back to Lobby</button>';
            gameArea.innerHTML = '<h2>Final Standings</h2>' + scoreStr;
            document.getElementById('lobby-btn').onclick = () => socket.send(JSON.stringify({ type: 'resetToLobby' }));
        }
    }
}

socket.addEventListener('close', () => console.log('Disconnected'));
