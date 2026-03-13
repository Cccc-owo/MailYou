export interface WindowControlsBridge {
  minimize(): Promise<void>
  toggleMaximize(): Promise<boolean>
  close(): Promise<void>
  isMaximized(): Promise<boolean>
  openExternal(url: string): Promise<void>
  focus(): Promise<void>
}
