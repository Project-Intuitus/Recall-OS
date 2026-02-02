import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import NotificationWindow from "./NotificationWindow";
import "./index.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60, // 1 minute
      retry: 1,
    },
  },
});

// Check if this is the notification window
const isNotificationWindow = window.location.pathname === "/notification" ||
  window.location.search.includes("notification=true");

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {isNotificationWindow ? (
      <NotificationWindow />
    ) : (
      <QueryClientProvider client={queryClient}>
        <App />
      </QueryClientProvider>
    )}
  </React.StrictMode>
);
