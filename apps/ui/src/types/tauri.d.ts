declare module "@tauri-apps/api/tauri" {
  export function invoke<T = unknown>(cmd: string, args?: any): Promise<T>;
}

declare module "@tauri-apps/plugin-dialog" {
  export interface OpenDialogOptions {
    multiple?: boolean;
    directory?: boolean;
    filters?: Array<{ name: string; extensions: string[] }>;
  }

  export interface MessageDialogOptions {
    title?: string;
    kind?: "info" | "warning" | "error";
  }

  export interface ConfirmDialogOptions {
    title?: string;
    kind?: "info" | "warning" | "error";
  }

  export function open(options?: OpenDialogOptions): Promise<string | string[] | null>;
  export function message(message: string, options?: MessageDialogOptions): Promise<void>;
  export function ask(message: string, options?: ConfirmDialogOptions): Promise<boolean>;
}
