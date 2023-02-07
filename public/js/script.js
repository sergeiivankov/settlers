window.addEventListener('load', () => {
  document.querySelector('.js-test-api').addEventListener('click', () => {
    fetch('/api/test').then(res => res.text()).then(text => alert(text));
  });

  let socket = null;

  document.querySelector('.js-open-ws').addEventListener('click', () => {
    socket = new WebSocket(
      `ws${location.protocol === 'https:' ? 's' : ''}://${location.host}/ws`
    );

    socket.onopen = () => {
      alert('Socket opened');
    };

    socket.onmessage = e => {
      alert(`Data received from server: ${e.data}`);
    };

    socket.onclose = e => {
      if (e.wasClean) {
        alert(`Connection closed cleanly, code=${e.code} reason=${e.reason}`);
      } else {
        alert('Connection died');
      }
    };

    socket.onerror = err => {
      console.log(err);
    };
  });

  document.querySelector('.js-send-ws').addEventListener('click', () => {
    if(!socket) return alert('Socket not opened');
    socket.send("My name is John");
  });

  document.querySelector('.js-close-ws').addEventListener('click', () => {
    if(!socket) return alert('Socket not opened');
    socket.close();
    socket = null;
  });
});