import { useState, useCallback } from "react";
import type { PREvent, StoredNotification } from "../types";
import { generateId } from "../types";

export function useNotifications() {
  const [notifications, setNotifications] = useState<StoredNotification[]>([]);

  const addNotification = useCallback((event: PREvent) => {
    setNotifications((prev) => [
      { id: generateId(), event, timestamp: new Date().toISOString(), read: false },
      ...prev,
    ]);
  }, []);

  const markRead = useCallback((id: string) => {
    setNotifications((prev) =>
      prev.map((n) => (n.id === id ? { ...n, read: true } : n)),
    );
  }, []);

  const markAllRead = useCallback(() => {
    setNotifications((prev) => prev.map((n) => ({ ...n, read: true })));
  }, []);

  const remove = useCallback((id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  }, []);

  const unreadCount = notifications.filter((n) => !n.read).length;

  return { notifications, addNotification, markRead, markAllRead, remove, unreadCount };
}
