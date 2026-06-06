var sessionToken = null;
var currentSecret = null;
var responseTimer = null;

function logError(msg) {
	var errBox = document.getElementById('error-log');
	errBox.style.display = 'block';
	var div = document.createElement('div');
	div.textContent = msg;
	errBox.appendChild(div);
}

window.addEventListener('error', function (e) {
	logError(e.filename + ':' + e.lineno + ' ' + e.message);
});

function ajax(method, url, data, cb) {
	var loader = document.getElementById('loader');
	loader.style.display = 'block';

	var xhr = new XMLHttpRequest();
	xhr.open(method, url, true);
	xhr.setRequestHeader('Content-Type', 'application/json');

	xhr.onload = function() {
		loader.style.display = 'none';
		if (xhr.status === 200) {
			try {
				var json = JSON.parse(xhr.responseText);
				cb(null, json);
			} catch(e) {
				cb('JSON Parse Error: ' + e.message);
			}
		} else {
			cb('HTTP Status ' + xhr.status + ': ' + xhr.responseText);
		}
	};

	xhr.onerror = function() {
		loader.style.display = 'none';
		cb('Network Request Failed');
	};

	if (data) {
		xhr.send(JSON.stringify(data));
	} else {
		xhr.send(null);
	}
}

function showScreen(screenId) {
	var screens = ['screen-login', 'screen-game', 'screen-result'];
	for (var i = 0; i < screens.length; i++) {
		document.getElementById(screens[i]).style.display = (screens[i] === screenId) ? 'block' : 'none';
	}
}

function handleGameState(state) {
	sessionToken = state.session_token;
	currentSecret = state.secret;

	document.getElementById('stat-level').textContent = state.level_name;
	document.getElementById('stat-points').textContent = state.total_points;
	document.getElementById('stat-streak').textContent = state.streak + ' 🔥';

	document.getElementById('prompt').textContent = state.prompt;
	var prompt_img = document.getElementById('prompt-img');
	if (state.image_url) {
		prompt_img.style.display = 'block';
		prompt_img.src = state.image_url;
	} else {
		prompt_img.style.display = 'none';
	}

	var btnHint = document.getElementById('btn-hint');
	if (state.hints.indexOf('F') !== -1) {
		btnHint.style.display = 'block';
	} else {
		btnHint.style.display = 'none';
	}

	if (state.is_spelling) {
		document.getElementById('choices-container').style.display = 'none';
		document.getElementById('spelling-container').style.display = 'block';
		document.getElementById('spelling-input').value = '';
	} else {
		document.getElementById('spelling-container').style.display = 'none';
		document.getElementById('choices-container').style.display = 'block';

		var container = document.getElementById('choices-container');
		container.innerHTML = '';

		for (var i = 0; i < state.choices.length; i++) {
			var choice = state.choices[i];
			var div = document.createElement('div');
			div.className = 'choice-box';
			div.setAttribute('data-nonce', choice.nonce);

			// 1. If an image URL is parsed, append an image node
			if (choice.image_url) {
				var img = document.createElement('img');
				img.className = 'choice-img';
				img.src = choice.image_url;
				div.appendChild(img);
			}

			// 2. Append text options if present (supports text-only and hybrid choice elements)
			if (choice.text && choice.text.trim() !== '') {
				var span = document.createElement('span');
				span.textContent = choice.text;
				div.appendChild(span);
			}

			div.onclick = function() {
				submitAnswer(this.getAttribute('data-nonce'));
			};
			container.appendChild(div);
		}
	}

	showScreen('screen-game');
	responseTimer = new Date().getTime();
}

function submitAnswer(answerStr) {
	var responseTime = new Date().getTime() - responseTimer;

	var payload = {
		session_token: sessionToken,
		answer: answerStr,
		response_time_ms: responseTime,
		secret: currentSecret
	};

	ajax('POST', '/api/answer', payload, function(err, res) {
		if (err) {
			logError('Answer Error: ' + err);
			return;
		}

		currentSecret = res.secret;

		document.getElementById('result-status').textContent = res.correct ? 'CORRECT!' : 'INCORRECT';
		document.getElementById('result-word').textContent = res.word;
		document.getElementById('result-definition').textContent = res.definition;
		document.getElementById('result-context').textContent = res.context;
		document.getElementById('result-points').textContent = '+' + res.points_earned + ' Points ' + res.progress + '% Mastery';

		// Update running stats
		document.getElementById('stat-points').textContent = res.total_points;

		showScreen('screen-result');
	});
}

window.onload = function() {
	var btnSignin = document.getElementById('btn-signin');
	var btnGuest = document.getElementById('btn-guest');
	var btnHint = document.getElementById('btn-hint');
	var btnSubmitSpelling = document.getElementById('btn-submit-spelling');
	var btnNext = document.getElementById('btn-next');

	btnSignin.onclick = function() {
		var email = document.getElementById('email').value;
		var password = document.getElementById('password').value;

		ajax('POST', '/api/start', { email: email, password: password, guest: false }, function(err, state) {
			if (err) {
				logError('Login/Start Failed: ' + err);
				return;
			}
			handleGameState(state);
		});
	};

	btnGuest.onclick = function() {
		ajax('POST', '/api/start', { guest: true }, function(err, state) {
			if (err) {
				logError('Guest Start Failed: ' + err);
				return;
			}
			handleGameState(state);
		});
	};

	btnHint.onclick = function() {
		btnHint.style.display = 'none';
		ajax('POST', '/api/hint', { session_token: sessionToken, secret: currentSecret }, function(err, res) {
			if (err) {
				logError('Hint retrieval failed: ' + err);
				return;
			}
			currentSecret = res.secret;

			// Remove the eliminated choices from the DOM
			var noncesToRemove = res.nonces_to_remove;
			var container = document.getElementById('choices-container');
			var boxes = container.getElementsByClassName('choice-box');

			// Iterate backward to avoid index shifting on element removal
			for (var i = boxes.length - 1; i >= 0; i--) {
				var nonce = boxes[i].getAttribute('data-nonce');
				if (noncesToRemove.indexOf(nonce) !== -1) {
					container.removeChild(boxes[i]);
				}
			}
		});
	};

	btnSubmitSpelling.onclick = function() {
		var guess = document.getElementById('spelling-input').value;
		if (guess.trim() !== '') {
			submitAnswer(guess);
		}
	};

	btnNext.onclick = function() {
		ajax('POST', '/api/next', { session_token: sessionToken, secret: currentSecret }, function(err, state) {
			if (err) {
				logError('Next Question Loading Failed: ' + err);
				return;
			}
			handleGameState(state);
		});
	};
};