import { invoke } from '@tauri-apps/api/core'

export interface CurseForgeMod {
	id: number
	gameId: number
	name: string
	slug: string
	summary: string
	classId: number
	links: { websiteUrl: string }
}

export interface CurseForgeSearchResponse {
	data: CurseForgeMod[]
	pagination: { index: number; pageSize: number; totalCount: number }
}

export interface CurseForgeSearchOptions {
	searchFilter?: string
	gameVersion?: string
	modLoaderTypes?: number[]
	sortField?: number
	sortOrder?: 'asc' | 'desc'
	pageSize?: number
	index?: number
}

function toQuery(values: Record<string, string | number | number[] | undefined>) {
	return Object.fromEntries(
		Object.entries(values)
			.filter(([, value]) => value !== undefined)
			.map(([key, value]) => [key, String(value)]),
	)
}

export function getCurseForgeStatus(): Promise<boolean> {
	return invoke<boolean>('plugin:curseforge|curseforge_status').catch(() => false)
}

export function curseforgeRequest<T>(
	method: 'GET' | 'POST',
	path: string,
	query?: Record<string, string>,
	body?: unknown,
): Promise<T> {
	return invoke('plugin:curseforge|curseforge_request', { method, path, query, body })
}

export function searchCurseForgeMods(options: CurseForgeSearchOptions = {}) {
	return curseforgeRequest<CurseForgeSearchResponse>(
		'GET',
		'mods/search',
		toQuery({ gameId: 432, classId: 6, ...options }),
	)
}
