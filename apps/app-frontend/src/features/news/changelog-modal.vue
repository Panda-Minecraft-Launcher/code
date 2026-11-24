<script setup lang="ts">
import { getChangelog } from '@modrinth/blog'
import { ChangelogEntry, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { onMounted, ref, useTemplateRef } from 'vue'

import { getAppVersion } from '@/platform/app-version'

const SEEN_VERSION_KEY = 'changelog_seen_for_version'

const { formatMessage } = useVIntl()
const messages = defineMessages({
	title: {
		id: 'app.changelog.title',
		defaultMessage: 'Release notes',
	},
	currentVersion: {
		id: 'app.changelog.current-version',
		defaultMessage: 'Current version: {version}',
	},
	youAreHere: {
		id: 'app.changelog.you-are-here',
		defaultMessage: 'You are here',
	},
})

const modal = useTemplateRef<InstanceType<typeof NewModal>>('modal')
const currentVersion = ref('')
const entries = getChangelog()
	.filter((entry) => entry.product === 'app' && entry.version)
	.slice(0, 2)

function show() {
	modal.value?.show()
}

function hide() {
	modal.value?.hide()
}

defineExpose({ show, hide })

onMounted(async () => {
	try {
		currentVersion.value = await getAppVersion()
		const seen = localStorage.getItem(SEEN_VERSION_KEY)
		localStorage.setItem(SEEN_VERSION_KEY, currentVersion.value)
		if (seen !== null && seen !== currentVersion.value) {
			show()
		}
	} catch {
		// Storage and version lookups must never block the app.
	}
})
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title)"
		:scrollable="true"
		max-width="720px"
	>
		<div class="flex flex-col gap-6">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.currentVersion, { version: currentVersion }) }}
			</p>
			<div v-for="entry in entries" :key="entry.version" class="flex flex-col gap-3">
				<span v-if="entry.version === currentVersion" class="ml-8 text-sm font-semibold text-brand">
					{{ formatMessage(messages.youAreHere) }}
				</span>
				<ChangelogEntry :entry="entry" />
			</div>
		</div>
	</NewModal>
</template>
