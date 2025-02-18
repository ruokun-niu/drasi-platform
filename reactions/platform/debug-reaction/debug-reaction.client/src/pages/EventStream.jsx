import React, { useState, useEffect } from "react";

function EventStream() {
    const [stream, setStream] = useState([]);
    const WEBSOCKET_URL = "ws://localhost:5195/ws/stream";

    const fetchStream = async () => {
        try {
            const url = "http://localhost:5195/stream";
            const response = await fetch(url);

            if (!response.ok) {
                throw new Error(`HTTP error! Status: ${response.status}`);
            }

            const stream = await response.json();
            console.log(stream);
            setStream(stream);
        } catch (error) {
            console.error("Error fetching stream:", error);
        }
    };
    useEffect(() => {
        const ws = new WebSocket(WEBSOCKET_URL);

        fetchStream();
        ws.onopen = () => {
            console.log("WebSocket connected");
            setInterval(() => {
                if (ws.readyState == WebSocket.OPEN) {
                    ws.send(JSON.stringify({ type: "ping" }));
                }
            }, 25000);
        };

        ws.onmessage = (event) => {
            try  {
                console.log("Message from server:", event.data);
                const data = JSON.parse(event.data);
                setStream(data);
            } catch (error) {
                console.error("Error parsing message:", error);
            }
        };

        ws.onerror = (error) => {
            console.error('WebSocket error:', error);
        };
  
        ws.onclose = (event) => {
          console.log("WebSocket closed:", event.code, event.reason);
        };

        // Closing the web socket on component unmount
        return () => {
            ws.close();
        }
    }, []);
    

    // useEffect(() => {
    //     fetchStream();
    // }, []);
    

    return (
        <div>
            <h2>Event Stream</h2>
            <div>
                {stream.map((event, index) => (
                    <div key={index} className="alert alert-secondary">
                        {JSON.stringify(event)}
                    </div>
                ))}
            </div>
        </div>
    );
}

export default EventStream;
