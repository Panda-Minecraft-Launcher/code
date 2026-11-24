<template>
	<CreationFlowModal
		ref="modal"
		type="instance"
		:available-loaders="['optifine']"
		:game-versions="gameVersions"
		:hide-loader-chips="true"
		:hide-loader-version-type-chips="true"
		:loader-version-label="optifineMessages.loaderVersion"
		:finish-disabled="installDisabled"
		:get-loader-manifest="getLoaderManifest"
		@create="handleCreate"
	/>
</template>

<script setup lang="ts">
import {
	type CreationFlowContextValue,
	CreationFlowModal,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed, nextTick, useTemplateRef } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import { optifineMessages } from '@/features/optifine/messages'
import {
	optifineGameVersionsQueryOptions,
	optifineKeys,
	optifineVersionsQueryOptions,
} from '@/features/optifine/queries'
import { wait_for_install_job } from '@/helpers/install'
import { install, type OptifineInstallResult } from '@/platform/app-lib/optifine/commands'
import { injectAppEvents } from '@/providers/app-events'

defineOptions({ name: 'OptifineInstallModal' })

const { formatMessage } = useVIntl()
const { addNotification, handleError } = injectNotificationManager()
const appEvents = injectAppEvents()
const queryClient = useQueryClient()
const modal = useTemplateRef<ComponentExposed<typeof CreationFlowModal>>('modal')

const gameVersionsQuery = useQuery(optifineGameVersionsQueryOptions())
const gameVersions = computed(() => gameVersionsQuery.data.value ?? [])
const versionsQuery = useQuery(optifineVersionsQueryOptions(gameVersions))
const versions = computed(() => versionsQuery.data.value ?? [])
const installDisabled = computed(
	() => gameVersions.value.length === 0 || versions.value.length === 0,
)

// BMCLAPI labels are not chronological; numeric label order puts the newest build first.
function getLoaderManifest() {
	const sortedVersions = [...versions.value].sort((a, b) =>
		b.label.localeCompare(a.label, undefined, { numeric: true }),
	)
	return Promise.resolve({
		gameVersions: gameVersions.value.map((gameVersion) => ({
			id: gameVersion,
			stable: true,
			loaders: sortedVersions
				.filter((version) => version.game_version === gameVersion)
				.map((version) => ({
					id: version.label,
					url: '',
					stable: !version.preview,
				})),
		})),
	})
}

async function show() {
	await gameVersionsQuery.suspense()
	await nextTick()
	await versionsQuery.suspense()
	await modal.value?.show((ctx) => {
		ctx.setSetupType('custom')
		ctx.selectedLoader.value = 'optifine'
		ctx.selectedGameVersion.value = gameVersions.value[0] ?? null
		ctx.loaderVersionType.value = 'stable'
	})
}

function getInstallJobId(result: OptifineInstallResult): string | null {
	if (typeof result === 'string') return result
	if ('job_id' in result && typeof result.job_id === 'string') return result.job_id
	return null
}

async function handleCreate(config: CreationFlowContextValue) {
	const gameVersion = config.selectedGameVersion.value
	if (!gameVersion) return

	config.loading.value = false
	modal.value?.hide()

	const loaderVersion = config.selectedLoaderVersion.value
	const selected = versions.value.find(
		(version) => version.label === loaderVersion || version.version === loaderVersion,
	)

	try {
		const result = await install(gameVersion, selected?.version)
		const jobId = getInstallJobId(result)
		if (jobId) await wait_for_install_job(appEvents, jobId)
		addNotification({
			type: 'success',
			title: formatMessage(optifineMessages.installed),
		})
		void queryClient.invalidateQueries({ queryKey: optifineKeys.instances() })
	} catch (error) {
		handleError(error as Error)
	}
}

defineExpose({ show })
</script>
