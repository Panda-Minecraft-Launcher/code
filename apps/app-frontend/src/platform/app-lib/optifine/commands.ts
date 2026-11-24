import { invoke } from '@tauri-apps/api/core'

import type { InstallJobSnapshot } from '@/helpers/install'
import type { GameInstance } from '@/helpers/types'

export type OptifineVersion = {
	game_version: string
	version: string
	label: string
	filename: string
	preview: boolean
	forge: string | null
}

export async function getGameVersions(): Promise<string[]> {
	return await invoke<string[]>('plugin:optifine|optifine_game_versions')
}

export async function getVersions(gameVersion: string): Promise<OptifineVersion[]> {
	return await invoke<OptifineVersion[]>('plugin:optifine|optifine_versions', { gameVersion })
}

export async function listInstances(): Promise<GameInstance[]> {
	const instances = await invoke<GameInstance[]>('plugin:instance|instance_list')
	return instances.filter((instance) => instance.loader === 'optifine')
}

export type OptifineInstallResult = GameInstance | InstallJobSnapshot | string

export async function install(
	gameVersion: string,
	version?: string,
): Promise<OptifineInstallResult> {
	return await invoke<OptifineInstallResult>('plugin:optifine|optifine_install', {
		input: {
			game_version: gameVersion,
			...(version ? { version } : {}),
		},
	})
}
