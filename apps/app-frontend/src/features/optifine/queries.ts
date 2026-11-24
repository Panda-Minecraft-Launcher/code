import { queryOptions } from '@tanstack/vue-query'
import { computed, type Ref } from 'vue'

import { getGameVersions, getVersions, listInstances } from '@/platform/app-lib/optifine/commands'

export const optifineKeys = {
	all: ['optifine'] as const,
	gameVersions: () => [...optifineKeys.all, 'game-versions'] as const,
	versions: () => [...optifineKeys.all, 'versions'] as const,
	instances: () => [...optifineKeys.all, 'instances'] as const,
}

export const optifineGameVersionsQueryOptions = () =>
	queryOptions({
		queryKey: optifineKeys.gameVersions(),
		queryFn: getGameVersions,
		staleTime: Infinity,
	})

export const optifineVersionsQueryOptions = (gameVersions: Ref<string[]>) =>
	queryOptions({
		queryKey: optifineKeys.versions(),
		enabled: computed(() => gameVersions.value.length > 0),
		queryFn: async () => {
			const versionsByGameVersion = await Promise.all(gameVersions.value.map(getVersions))
			return versionsByGameVersion.flat()
		},
		staleTime: Infinity,
	})

export const optifineInstancesQueryOptions = () =>
	queryOptions({
		queryKey: optifineKeys.instances(),
		queryFn: listInstances,
		networkMode: 'always',
		staleTime: 30_000,
	})
