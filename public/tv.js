const socket = new WebSocket(`ws://${window.location.host}/ws`);

const lobbyScreen = document.getElementById('lobby-screen');
const gameArea = document.getElementById('game-area');
const playerList = document.getElementById('player-list');
const timerDisplay = document.getElementById('timer-display');

let currentPhase = 'lobby';
let playerMap = {};

socket.addEventListener('message', (event) => {
    const msg = JSON.parse(event.data);
    console.log('TV received:', msg);

    if (msg.timer !== undefined) {
        timerDisplay.innerText = msg.timer;
        timerDisplay.style.display = (msg.timer > 0 && msg.phase !== 'lobby') ? 'block' : 'none';
    }

    switch (msg.type) {
        case 'welcome':
        case 'lobbyState':
            currentPhase = msg.phase;
            updateUI(msg.players);
            break;
        case 'gameState':
            currentPhase = msg.phase;
            lobbyScreen.style.display = currentPhase === 'lobby' ? 'block' : 'none';
            gameArea.style.display = currentPhase !== 'lobby' ? 'block' : 'none';
            updateSpectator(msg);
            break;
    }
});

function updateUI(players) {
    playerMap = {};
    players.forEach(p => playerMap[p.id] = p.name);
    
    playerList.innerHTML = '';
    players.forEach((player) => {
        const li = document.createElement('li');
        li.textContent = player.name;
        playerList.appendChild(li);
    });
}

function updateSpectator(msg) {
    const phase = msg.phase;
    const currentQuestion = msg.currentQuestion !== undefined ? msg.currentQuestion : msg.current_question;

    if (phase === 'prompting') {
        gameArea.innerHTML = '<h2 class="big-prompt">Answering Prompts...</h2><p>Check your phones!</p>';
    } else if (phase === 'voting') {
        if (currentQuestion) {
            gameArea.innerHTML = `<h2>Time to Vote!</h2><p class="prompt-text big-prompt">${currentQuestion.prompt}</p>`;
            gameArea.innerHTML += '<div class="voting-preview"></div>';
            const preview = gameArea.querySelector('.voting-preview');
            for (const [id, answer] of Object.entries(currentQuestion.answers)) {
                const div = document.createElement('div');
                div.className = 'vote-button-disabled';
                div.style.fontSize = '2.5rem';
                div.textContent = answer;
                preview.appendChild(div);
            }
        }
    } else if (phase === 'reveal') {
        if (currentQuestion) {
            gameArea.innerHTML = `<h2>Results</h2><p class="prompt-text big-prompt">${currentQuestion.prompt}</p>`;
            const container = document.createElement('div');
            container.className = 'reveal-container';
            
            for (const [id, answer] of Object.entries(currentQuestion.answers)) {
                const name = playerMap[id] || 'Unknown';
                const votes = Object.values(currentQuestion.votes).filter(v => v === parseInt(id)).length;
                container.innerHTML += `
                    <div class="reveal-card">
                        <div class="reveal-answer" style="font-size: 3rem;">"${answer}"</div>
                        <div class="reveal-name" style="font-size: 2rem;">- ${name}</div>
                        <div class="reveal-votes" style="font-size: 2.5rem;">${votes} Votes</div>
                    </div>
                `;
            }
            gameArea.appendChild(container);
        }
    } else if (phase === 'results') {
        gameArea.innerHTML = '<h2>Final Standings</h2><div id="score-display"></div>';
        const display = document.getElementById('score-display');
        let scoreStr = '<ul style="flex-direction: column; align-items: center;">';
        const sorted = Object.entries(msg.scores).sort((a, b) => b[1] - a[1]);
        sorted.forEach(([id, score]) => {
            const name = playerMap[id] || `Player ${id}`;
            scoreStr += `<li style="width: 80%; justify-content: space-between;"><span>${name}</span><span>${score} pts</span></li>`;
        });
        scoreStr += '</ul>';
        display.innerHTML = scoreStr;
    }
}
