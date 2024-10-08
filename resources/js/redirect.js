function createWebSocketConnection(url) {
    // Create a WebSocket connection to the server
    const socket = new WebSocket(url);

    // Event listener for when the WebSocket connection is opened
    socket.addEventListener('open', function (event) {
        console.log('WebSocket connection opened:', event);
    });

    // Event listener for when a message is received from the WebSocket
    socket.addEventListener('message', function (event) {
        console.log('Message from server:', event.data);

        var event = JSON.parse(event.data);

        // Check if the message contains the event that should trigger a redirect
        if (event.type === 'redirect_event') {
            console.log('Received redirect event:', event);
            socket.send('Received redirect event');
            // Redirect to the new page
            window.location.href = event.url;
        }
    });

    // Event listener for when the WebSocket connection is closed
    socket.addEventListener('close', function (event) {
        console.log('WebSocket connection closed:', event);
    });

    // Event listener for when an error occurs with the WebSocket
    socket.addEventListener('error', function (event) {
        console.error('WebSocket error:', event);
    });
}
