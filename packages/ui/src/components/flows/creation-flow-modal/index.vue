<template>
	<MultiStageModal
		ref="modal"
		:stages="ctx.stageConfigs"
		:context="ctx"
		:fade="fade"
		:animate-stages="type === 'server-onboarding'"
		:disable-close="ctx.navigating.value"
		disable-progress
		@hide="handleHide"
		@after-hide="emit('after-hide')"
		@after-show="emit('after-show')"
	/>
</template>

<script setup lang="ts">
import { computed, useTemplateRef } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import type { MessageDescriptor } from '#ui/composables/i18n'

import MultiStageModal from '../../base/MultiStageModal.vue'
import {
	createCreationFlowContext,
	type CreationFlowContextValue,
	type CurseForgeTarget,
	type FlowType,
	type LoaderManifestResolver,
	type ProjectInstallCreateData,
	type ProjectInstallSelection,
	type ProjectSearchResult,
	provideCreationFlowContext,
} from './creation-flow-context'

const props = withDefaults(
	defineProps<{
		type?: FlowType
		availableLoaders?: string[]
		showSnapshotToggle?: boolean
		disableClose?: boolean
		isInitialSetup?: boolean
		initialLoader?: string
		initialGameVersion?: string
		gameVersions?: string[]
		hideLoaderChips?: boolean
		hideLoaderVersionTypeChips?: boolean
		loaderVersionLabel?: MessageDescriptor
		fetchExistingInstanceNames?: () => Promise<string[]>
		onBack?: (() => boolean | void) | null
		browseModpacks?: () => Promise<void>
		fade?: 'standard' | 'warning' | 'danger'
		searchProjects?: (query: string, limit?: number) => Promise<ProjectSearchResult>
		prepareProjectInstall?: (
			projectId: string,
			projectType: string,
		) => Promise<ProjectInstallSelection | null>
		createProjectInstall?: (data: ProjectInstallCreateData) => Promise<void>
		getProjectVersions?: (projectId: string) => Promise<{ id: string }[]>
		getLoaderManifest?: LoaderManifestResolver
		getCurseForgeTarget?: () => Promise<CurseForgeTarget>
		randomizeInstanceIcon?: () => Promise<{ path: string; previewUrl: string } | null>
		customizeInstanceIcon?: () => void
		finishDisabled?: boolean
		finishDisabledTooltip?: string
	}>(),
	{
		type: 'world',
		availableLoaders: () => ['fabric', 'neoforge', 'forge', 'quilt'],
		showSnapshotToggle: false,
		disableClose: false,
		isInitialSetup: false,
		initialLoader: undefined,
		initialGameVersion: undefined,
		gameVersions: undefined,
		hideLoaderChips: undefined,
		hideLoaderVersionTypeChips: false,
		loaderVersionLabel: undefined,
		fetchExistingInstanceNames: undefined,
		onBack: null,
		browseModpacks: undefined,
		randomizeInstanceIcon: undefined,
		customizeInstanceIcon: undefined,
	},
)

const emit = defineEmits<{
	(e: 'hide' | 'after-hide' | 'after-show' | 'browse-modpacks'): void
	(e: 'create', config: CreationFlowContextValue): void
}>()

const modal = useTemplateRef<ComponentExposed<typeof MultiStageModal>>('modal')

const ctx = createCreationFlowContext(
	modal,
	props.type,
	{
		browseModpacks: () => emit('browse-modpacks'),
		create: (config) => emit('create', config),
	},
	{
		availableLoaders: props.availableLoaders,
		showSnapshotToggle: props.showSnapshotToggle,
		disableClose: props.disableClose,
		isInitialSetup: props.isInitialSetup,
		initialLoader: props.initialLoader,
		initialGameVersion: props.initialGameVersion,
		gameVersions: props.gameVersions,
		hideLoaderChips: props.hideLoaderChips,
		hideLoaderVersionTypeChips: props.hideLoaderVersionTypeChips,
		loaderVersionLabel: props.loaderVersionLabel,
		fetchExistingInstanceNames: props.fetchExistingInstanceNames,
		onBack: props.onBack ?? undefined,
		browseModpacks: props.browseModpacks,
		searchProjects: props.searchProjects,
		prepareProjectInstall: props.prepareProjectInstall,
		createProjectInstall: props.createProjectInstall,
		getProjectVersions: props.getProjectVersions,
		getLoaderManifest: props.getLoaderManifest,
		getCurseForgeTarget: props.getCurseForgeTarget,
		randomizeInstanceIcon: props.randomizeInstanceIcon,
		customizeInstanceIcon: props.customizeInstanceIcon,
		finishDisabled: computed(() => props.finishDisabled ?? false),
		finishDisabledTooltip: computed(() => props.finishDisabledTooltip),
	},
)
provideCreationFlowContext(ctx)

async function show(prepare?: (context: CreationFlowContextValue) => void | Promise<void>) {
	await ctx.reset()
	void ctx.prefetchLoaderMetadata()
	modal.value?.setStage(0)
	await prepare?.(ctx)
	modal.value?.show()
}

function hide() {
	modal.value?.hide()
}

function handleHide() {
	ctx.cancelBackup.value?.()
	emit('hide')
}

defineExpose({ show, hide, ctx })
</script>
