import { useState, useEffect, useRef } from 'react';
import type { McpLog } from '../types';

export function useWebSocket(url: string): McpLog[] {
  const [events, setEvents] = useState<McpLog[]>([]);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | null>(null);

  useEffect(() => {
    let mounted = true;

    const connect = () => {
      if (!mounted) return;

      try {
        const ws = new WebSocket(url);
        wsRef.current = ws;

        ws.onopen = () => {
          console.log('WebSocket connected');
          if (reconnectTimeoutRef.current) {
            clearTimeout(reconnectTimeoutRef.current);
            reconnectTimeoutRef.current = null;
          }
        };

        ws.onmessage = (event) => {
          try {
            const data: McpLog = JSON.parse(event.data);
            setEvents((prev) => [...prev, data].slice(-1000)); // Keep last 1000 events
          } catch (e) {
            console.error('Failed to parse WebSocket message:', e);
          }
        };

        ws.onerror = (error) => {
          console.error('WebSocket error:', error);
        };

        ws.onclose = () => {
          console.log('WebSocket disconnected');
          if (mounted) {
            reconnectTimeoutRef.current = window.setTimeout(() => {
              connect();
            }, 3000);
          }
        };
      } catch (e) {
        console.error('Failed to create WebSocket:', e);
        if (mounted) {
          reconnectTimeoutRef.current = window.setTimeout(() => {
            connect();
          }, 3000);
        }
      }
    };

    connect();

    return () => {
      mounted = false;
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, [url]);

  return events;
}

