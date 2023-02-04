window.addEventListener('load', () => {
  document.querySelector('.js-test-api').addEventListener('click', () => {
    fetch('/api/test').then(res => res.text()).then(text => alert(text));
  });

  document.querySelector('.js-test-ws').addEventListener('click', () => {
    let socket = new WebSocket(
      `ws${location.protocol === 'https:' ? 's' : ''}://${location.host}/ws`
    );

    socket.onopen = () => {
      socket.send("My name is John");
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
});