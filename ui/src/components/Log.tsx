import React, { useState, useCallback, useEffect } from 'react';
import useWebSocket from 'react-use-websocket';
import styled from 'styled-components';

const Line = styled.pre`
    display: block;
    margin-bottom: 0;
`;

const Log = ({ws}: {ws: string}) => {
  let url = new URL(ws, window.location.href);
  url.protocol = 'ws:';

  const [messageHistory, setMessageHistory] = useState<MessageEvent<any>[]>([]);

  const { sendMessage, lastMessage, readyState } = useWebSocket(url.href);

  useEffect(() => {
    if (lastMessage !== null) {
      setMessageHistory((prev: MessageEvent<any>[]) => prev.concat(lastMessage));
    }
  }, [lastMessage, setMessageHistory]);

  return (
    <div>
        {messageHistory.map((message, idx) => (
            <Line key={idx}>{message ? message.data : null}</Line>
        ))}
    </div>
  );
};

export default Log;