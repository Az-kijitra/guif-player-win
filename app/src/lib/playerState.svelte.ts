import { api } from "./api";
import type {
  AlbumSummary,
  AudioTrack,
  FolderContent,
  PlaybackState,
  Playlist,
  TagUpdateRequest,
  TagUpdateResult,
  ThemePreset,
  ThemeSetting,
  TrackResume,
  ViewMode,
} from "./types";

const THEME_STORAGE_KEY = "guif_player_theme_setting";
const RESOLVED_THEME_STORAGE_KEY = "guif_player_resolved_theme";

const ERROR_DISPLAY_MS = 8000;

const VALID_PRESETS: ThemePreset[] = [
  "nord",
  "default",
  "monokai",
  "catppuccin",
  "solarized",
  "light",
];

function applyThemeToDom(preset: ThemePreset) {
  if (typeof document !== "undefined") {
    document.documentElement.dataset.theme = preset;
  }
}

export function formatTime(ms: number): string {
  if (isNaN(ms) || ms < 0) return "00:00";
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

class PlayerStore {
  currentPath = $state<string>("");
  drives = $state<string[]>([]);
  folderContent = $state<FolderContent | null>(null);
  albums = $state<AlbumSummary[]>([]);
  playlists = $state<Playlist[]>([]);
  activePlaylistId = $state<number | null>(null);
  playlistTracks = $state<AudioTrack[]>([]);
  trackResumes = $state<Record<string, TrackResume>>({});
  viewMode = $state<ViewMode>("FOLDER");
  showDetailDialog = $state<boolean>(false);
  playlistModalTrack = $state<AudioTrack | null>(null);
  currentArtwork = $state<string | null>(null);

  // Property Inspector & Selection state (Pattern B)
  showInspector = $state<boolean>(true);
  selectedTracks = $state<AudioTrack[]>([]);
  lastSelectedPath = $state<string | null>(null);
  lastPlayedFilePath = $state<string | null>(null);
  artworkVersion = $state<number>(0);

  // Theme state (guif aligned)
  themeSetting = $state<ThemeSetting>("nord");
  resolvedTheme = $state<ThemePreset>("nord");

  constructor() {
    if (typeof localStorage !== "undefined") {
      const cachedSetting = localStorage.getItem(THEME_STORAGE_KEY) as ThemeSetting | null;
      const cachedResolved = localStorage.getItem(RESOLVED_THEME_STORAGE_KEY) as ThemePreset | null;
      if (cachedSetting) {
        this.themeSetting = cachedSetting;
      }
      if (cachedResolved && VALID_PRESETS.includes(cachedResolved)) {
        this.resolvedTheme = cachedResolved;
        applyThemeToDom(cachedResolved);
      }
    }
  }

  playback = $state<PlaybackState>({
    current_track: null,
    is_playing: false,
    position_ms: 0,
    duration_ms: 0,
    volume: 1.0,
    shuffle_mode: false,
    repeat_mode: 0,
    queue: [],
  });

  errorMessage = $state<string | null>(null);
  private errorTimer: ReturnType<typeof setTimeout> | null = null;

  private syncInterval: number | null = null;
  private lastArtworkTrackPath: string | null = null;

  showError(message: string) {
    if (this.errorTimer) clearTimeout(this.errorTimer);
    this.errorMessage = message;
    this.errorTimer = setTimeout(() => this.clearError(), ERROR_DISPLAY_MS);
  }

  clearError() {
    if (this.errorTimer) clearTimeout(this.errorTimer);
    this.errorTimer = null;
    this.errorMessage = null;
  }

  private reportError(context: string, e: unknown) {
    console.error(`${context}:`, e);
    this.showError(`${context}: ${e}`);
  }

  // The backend's playback messages are written for the screen
  private async attempt(action: () => Promise<unknown>) {
    try {
      await action();
    } catch (e) {
      this.showError(String(e));
    }
  }

  async init() {
    try {
      await api.onPlaybackError((message) => this.showError(message));
      await this.loadTheme();
      this.drives = await api.listDrives();
      const latestResume = await api.getLatestAlbumResume();
      const defaultPath = await api.getDefaultPath();
      this.lastPlayedFilePath = latestResume?.last_file_path ?? null;
      await this.navigateTo(defaultPath);

      if (latestResume) {
        const matchedItem = this.folderContent?.items.find(
          (i) => !i.is_directory && i.path === latestResume.last_file_path && i.track !== null
        );
        if (matchedItem?.track) {
          this.selectSingleTrack(matchedItem.track);
        }
      }

      await this.loadPlaylists();
      await this.loadResumes();

      // Poll playback state every 250ms
      this.startSync();
    } catch (e) {
      this.reportError("起動時の読み込みに失敗しました", e);
    }
  }

  startSync() {
    if (this.syncInterval) clearInterval(this.syncInterval);
    this.syncInterval = window.setInterval(async () => {
      try {
        const state = await api.getPlaybackState();
        this.playback = state;

        if (state.current_track?.file_path !== this.lastArtworkTrackPath) {
          this.lastArtworkTrackPath = state.current_track?.file_path ?? null;
          if (state.current_track) {
            this.currentArtwork = await api.getArtwork(state.current_track.file_path);
          } else {
            this.currentArtwork = null;
          }
        }
      } catch {
        // Ignored during shutdown or hot reload
      }
    }, 250);
  }

  async navigateTo(path: string) {
    this.currentPath = path;
    try {
      this.folderContent = await api.listFolder(path);
      await this.loadResumes();
    } catch (e) {
      this.reportError("フォルダーを開けません", e);
    }
  }

  async navigateUp() {
    if (this.folderContent?.parent_path) {
      await this.navigateTo(this.folderContent.parent_path);
    }
  }

  async loadAlbums() {
    if (!this.currentPath) return;
    try {
      this.albums = await api.listAlbums(this.currentPath);
    } catch (e) {
      this.reportError("アルバムを読み込めません", e);
    }
  }

  async loadPlaylists() {
    try {
      this.playlists = await api.getPlaylists();
    } catch (e) {
      this.reportError("プレイリストを読み込めません", e);
    }
  }

  async selectPlaylist(id: number) {
    this.activePlaylistId = id;
    try {
      this.playlistTracks = await api.getPlaylistTracks(id);
    } catch (e) {
      this.reportError("プレイリストの曲を読み込めません", e);
    }
  }

  async createPlaylist(name: string) {
    if (!name.trim()) return;
    try {
      const p = await api.createPlaylist(name.trim());
      await this.loadPlaylists();
      await this.selectPlaylist(p.id);
    } catch (e) {
      this.reportError("プレイリストを作成できません", e);
    }
  }

  async deletePlaylist(id: number) {
    try {
      await api.deletePlaylist(id);
      if (this.activePlaylistId === id) {
        this.activePlaylistId = null;
        this.playlistTracks = [];
      }
      await this.loadPlaylists();
    } catch (e) {
      this.reportError("プレイリストを削除できません", e);
    }
  }

  async addTrackToPlaylist(playlistId: number, track: AudioTrack) {
    try {
      await api.addToPlaylist(playlistId, track);
      await this.loadPlaylists();
      if (this.activePlaylistId === playlistId) {
        await this.selectPlaylist(playlistId);
      }
      this.playlistModalTrack = null;
    } catch (e) {
      this.reportError("プレイリストに追加できません", e);
    }
  }

  async removeTrackFromPlaylist(playlistId: number, filePath: string) {
    try {
      await api.removeFromPlaylist(playlistId, filePath);
      await this.loadPlaylists();
      await this.selectPlaylist(playlistId);
    } catch (e) {
      this.reportError("プレイリストから外せません", e);
    }
  }

  async loadResumes() {
    try {
      const list = await api.getAllTrackResumes();
      const map: Record<string, TrackResume> = {};
      for (const item of list) {
        map[item.file_path] = item;
      }
      this.trackResumes = map;
    } catch (e) {
      this.reportError("再生位置を読み込めません", e);
    }
  }

  async playTrackInList(track: AudioTrack, trackList: AudioTrack[]) {
    try {
      const index = trackList.findIndex((t) => t.file_path === track.file_path);
      const resume = this.trackResumes[track.file_path];
      const initialPos = resume && !resume.is_completed ? resume.position_ms : 0;

      await api.playQueue(trackList, Math.max(0, index), initialPos);
      this.playback = await api.getPlaybackState();
    } catch (e) {
      this.showError(String(e));
    }
  }

  async togglePlayPause() {
    await this.attempt(() => api.togglePlayPause());
  }

  async playNext() {
    await this.attempt(() => api.playNext());
  }

  async playPrevious() {
    await this.attempt(() => api.playPrevious());
  }

  async seek(posMs: number) {
    await this.attempt(() => api.seek(posMs));
  }

  async setVolume(vol: number) {
    await api.setVolume(vol);
  }

  async toggleShuffle() {
    await api.toggleShuffle();
  }

  async toggleRepeat() {
    await api.toggleRepeat();
  }

  setViewMode(mode: ViewMode) {
    this.viewMode = mode;
    if (mode === "ARTWORK") {
      this.loadAlbums();
    } else if (mode === "PLAYLIST") {
      this.loadPlaylists();
    }
  }

  isSelected(filePath: string): boolean {
    return this.selectedTracks.some((t) => t.file_path === filePath);
  }

  selectSingleTrack(track: AudioTrack) {
    this.selectedTracks = [track];
    this.lastSelectedPath = track.file_path;
  }

  toggleTrackSelection(track: AudioTrack) {
    const exists = this.selectedTracks.some((t) => t.file_path === track.file_path);
    if (exists) {
      this.selectedTracks = this.selectedTracks.filter(
        (t) => t.file_path !== track.file_path
      );
    } else {
      this.selectedTracks = [...this.selectedTracks, track];
    }
    this.lastSelectedPath = track.file_path;
  }

  selectRange(targetTrack: AudioTrack, allTracks: AudioTrack[]) {
    if (!this.lastSelectedPath) {
      this.selectSingleTrack(targetTrack);
      return;
    }
    const startIdx = allTracks.findIndex((t) => t.file_path === this.lastSelectedPath);
    const endIdx = allTracks.findIndex((t) => t.file_path === targetTrack.file_path);
    if (startIdx === -1 || endIdx === -1) {
      this.selectSingleTrack(targetTrack);
      return;
    }
    const min = Math.min(startIdx, endIdx);
    const max = Math.max(startIdx, endIdx);
    const rangeSlice = allTracks.slice(min, max + 1);

    // Merge keeping existing selection order + newly added in range
    const existingMap = new Set(this.selectedTracks.map((t) => t.file_path));
    const merged = [...this.selectedTracks];
    for (const t of rangeSlice) {
      if (!existingMap.has(t.file_path)) {
        merged.push(t);
      }
    }
    this.selectedTracks = merged;
    this.lastSelectedPath = targetTrack.file_path;
  }

  /** Shift + movement keys: `from` and `to` are null on rows that are not tracks. */
  extendSelection(from: AudioTrack | null, to: AudioTrack | null, allTracks: AudioTrack[]) {
    if (from && !this.isSelected(from.file_path)) {
      this.toggleTrackSelection(from);
    }
    if (to) {
      this.selectRange(to, allTracks);
    }
    this.showInspector = true;
  }

  selectAllTracks(allTracks: AudioTrack[]) {
    this.selectedTracks = [...allTracks];
    if (allTracks.length > 0) {
      this.lastSelectedPath = allTracks[allTracks.length - 1].file_path;
    }
  }

  clearSelection() {
    this.selectedTracks = [];
    this.lastSelectedPath = null;
  }

  moveSelectedTrack(index: number, direction: -1 | 1) {
    const targetIdx = index + direction;
    if (targetIdx < 0 || targetIdx >= this.selectedTracks.length) return;
    const copy = [...this.selectedTracks];
    const [moved] = copy.splice(index, 1);
    copy.splice(targetIdx, 0, moved);
    this.selectedTracks = copy;
  }

  async updateTracksMetadata(request: TagUpdateRequest): Promise<TagUpdateResult> {
    const result = await api.updateTracksMetadata(request);
    // Nothing changed: leave the form as it is, so the user can retry
    if (result.updated.length === 0) return result;
    const updatedMap = new Map(result.updated.map((t) => [t.file_path, t]));

    // Update selectedTracks
    this.selectedTracks = this.selectedTracks.map(
      (t) => updatedMap.get(t.file_path) ?? t
    );

    // Refresh current folder & playlist
    if (this.currentPath) {
      this.folderContent = await api.listFolder(this.currentPath);
    }
    if (this.viewMode === "ARTWORK" && this.currentPath) {
      this.albums = await api.listAlbums(this.currentPath);
    }
    if (this.activePlaylistId !== null) {
      this.playlistTracks = await api.getPlaylistTracks(this.activePlaylistId);
    }

    // Refresh current playing track artwork if it was modified
    if (
      this.playback.current_track &&
      updatedMap.has(this.playback.current_track.file_path)
    ) {
      this.currentArtwork = await api.getArtwork(
        this.playback.current_track.file_path
      );
    }
    this.artworkVersion += 1;

    return result;
  }

  async loadTheme() {
    try {
      const setting = (await api.getThemeSetting()) as ThemeSetting;
      this.themeSetting = setting;
      let resolved: ThemePreset = "nord";
      if (setting === "guif") {
        const guifPreset = (await api.getGuifThemePreset()) as ThemePreset;
        resolved = VALID_PRESETS.includes(guifPreset) ? guifPreset : "nord";
      } else if (VALID_PRESETS.includes(setting as ThemePreset)) {
        resolved = setting as ThemePreset;
      }
      this.resolvedTheme = resolved;
      applyThemeToDom(resolved);
      if (typeof localStorage !== "undefined") {
        localStorage.setItem(THEME_STORAGE_KEY, setting);
        localStorage.setItem(RESOLVED_THEME_STORAGE_KEY, resolved);
      }
    } catch (e) {
      this.reportError("テーマを読み込めません", e);
    }
  }

  async setTheme(setting: ThemeSetting) {
    this.themeSetting = setting;
    try {
      const resolvedStr = await api.saveThemeSetting(setting);
      const resolved: ThemePreset = VALID_PRESETS.includes(resolvedStr as ThemePreset)
        ? (resolvedStr as ThemePreset)
        : "nord";
      this.resolvedTheme = resolved;
      applyThemeToDom(resolved);
      if (typeof localStorage !== "undefined") {
        localStorage.setItem(THEME_STORAGE_KEY, setting);
        localStorage.setItem(RESOLVED_THEME_STORAGE_KEY, resolved);
      }
    } catch (e) {
      this.reportError("テーマを保存できません", e);
    }
  }
}

export const playerState = new PlayerStore();
