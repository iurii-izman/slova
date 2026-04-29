import { createSignal, createEffect, batch } from "solid-js";
import { createStore } from "solid-js/store";
import type { Job, JobId, JobState, JobFilter, AppEvent } from "../ipc/types";
import * as commands from "../ipc/commands";
import { onAppEvent } from "../ipc/commands";
import { extractErrorMessage } from "../utils/errors";

export type QueueStore = {
  jobs: Job[];
  filter: "all" | "active" | "failed" | "done";
  isLoading: boolean;
  error: string | null;
};

// Main store
const [queueState, setQueueState] = createStore<QueueStore>({
  jobs: [],
  filter: "all",
  isLoading: false,
  error: null,
});

let eventUnsubscribe: (() => Promise<void>) | null = null;

/**
 * Initialize the queue store: load jobs and subscribe to events
 */
export async function initQueueStore() {
  try {
    setQueueState("isLoading", true);
    setQueueState("error", null);

    // Load initial jobs
    const jobs = await commands.listJobs();
    setQueueState("jobs", jobs);

    // Subscribe to events
    eventUnsubscribe = onAppEvent((event) => {
      handleAppEvent(event);
    });
  } catch (err) {
    const message = extractErrorMessage(err);
    setQueueState("error", message);
  } finally {
    setQueueState("isLoading", false);
  }
}

/**
 * Clean up subscriptions
 */
export async function destroyQueueStore() {
  if (eventUnsubscribe) {
    await eventUnsubscribe();
    eventUnsubscribe = null;
  }
}

/**
 * Handle backend events and update store
 */
function handleAppEvent(event: AppEvent) {
  batch(() => {
    switch (event.type) {
      case "queue:tick": {
        // Batch update jobs by id
        const updates = event.payload.updates;
        for (const update of updates) {
          const idx = queueState.jobs.findIndex((j) => j.id === update.id);
          if (idx >= 0) {
            setQueueState("jobs", idx, "state", update.state);
          }
        }
        break;
      }

      case "job:done": {
        const idx = queueState.jobs.findIndex((j) => j.id === event.payload.id);
        if (idx >= 0) {
          setQueueState("jobs", idx, "state", event.payload.state);
        }
        break;
      }

      case "job:failed": {
        const idx = queueState.jobs.findIndex((j) => j.id === event.payload.id);
        if (idx >= 0) {
          setQueueState("jobs", idx, "state", event.payload.state);
        }
        break;
      }

      case "job:cancelled": {
        const idx = queueState.jobs.findIndex((j) => j.id === event.payload.id);
        if (idx >= 0) {
          setQueueState("jobs", idx, "state", event.payload.state);
        }
        break;
      }

      case "app:error": {
        if (event.payload.job_id) {
          const idx = queueState.jobs.findIndex((j) => j.id === event.payload.job_id);
          if (idx >= 0) {
            const errorState: JobState = {
              kind: "Failed",
              data: { error: event.payload.error, attempts: 0 },
            };
            setQueueState("jobs", idx, "state", errorState);
          }
        }
        // Also set global error
        setQueueState("error", event.payload.error.message);
        break;
      }

      // Other events are handled elsewhere (auth, rate limit)
      default:
        break;
    }
  });
}

/**
 * Add files to queue
 */
export async function addFilesToQueue(paths: string[]) {
  try {
    setQueueState("error", null);
    const jobIds = await commands.enqueueFiles(paths);
    // Reload jobs to get new entries
    const jobs = await commands.listJobs();
    setQueueState("jobs", jobs);
    return jobIds;
  } catch (err) {
    const message = extractErrorMessage(err);
    setQueueState("error", message);
    throw err;
  }
}

/**
 * Cancel a job
 */
export async function cancelJob(id: JobId) {
  try {
    setQueueState("error", null);
    await commands.cancelJob(id);
  } catch (err) {
    const message = extractErrorMessage(err);
    setQueueState("error", message);
    throw err;
  }
}

/**
 * Retry a failed job
 */
export async function retryJob(id: JobId) {
  try {
    setQueueState("error", null);
    await commands.retryJob(id);
  } catch (err) {
    const message = extractErrorMessage(err);
    setQueueState("error", message);
    throw err;
  }
}

/**
 * Pause the queue
 */
export async function pauseQueue() {
  try {
    setQueueState("error", null);
    await commands.pauseQueue();
  } catch (err) {
    const message = extractErrorMessage(err);
    setQueueState("error", message);
    throw err;
  }
}

/**
 * Resume the queue
 */
export async function resumeQueue() {
  try {
    setQueueState("error", null);
    await commands.resumeQueue();
  } catch (err) {
    const message = extractErrorMessage(err);
    setQueueState("error", message);
    throw err;
  }
}

/**
 * Set filter
 */
export function setFilter(filter: "all" | "active" | "failed" | "done") {
  setQueueState("filter", filter);
}

/**
 * Get filtered jobs
 */
export function getFilteredJobs(): Job[] {
  const filter = queueState.filter;

  if (filter === "all") {
    return queueState.jobs;
  }

  if (filter === "active") {
    return queueState.jobs.filter(
      (j) => j.state.kind !== "Done" && j.state.kind !== "Failed" && j.state.kind !== "Cancelled",
    );
  }

  if (filter === "failed") {
    return queueState.jobs.filter((j) => j.state.kind === "Failed");
  }

  if (filter === "done") {
    return queueState.jobs.filter((j) => j.state.kind === "Done");
  }

  return queueState.jobs;
}

// Accessor functions
export function getQueueState() {
  return queueState;
}

export function getJob(id: JobId): Job | undefined {
  return queueState.jobs.find((j) => j.id === id);
}

export function getJobs() {
  return queueState.jobs;
}

export function getFilter() {
  return queueState.filter;
}

export function getError() {
  return queueState.error;
}

export function isLoading() {
  return queueState.isLoading;
}
