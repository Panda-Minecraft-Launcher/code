<script setup lang="ts">
import { TriangleAlertIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	noDataCollection: {
		id: 'app.settings.privacy.no-data-collection',
		defaultMessage: 'We do not collect any data. We have no server that could collect it.',
	},
})
</script>

<template>
	<div class="flex flex-col items-center justify-center gap-4 py-16">
		<TriangleAlertIcon
			aria-hidden="true"
			class="h-[200px] w-[200px] text-[color:var(--color-yellow)]"
		/>
		<p>{{ formatMessage(messages.noDataCollection) }}</p>
	</div>
</template>
