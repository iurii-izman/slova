declare module '@tauri-apps/api/tauri' {
  export function invoke<T = unknown>(cmd: string, args?: any): Promise<T>;
}
