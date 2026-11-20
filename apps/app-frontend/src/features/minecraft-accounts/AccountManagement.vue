<template>
	<section class="flex flex-col">
		<h2 class="m-0 text-lg font-semibold text-contrast">
			{{ formatMessage(messages.minecraftAccount) }}
		</h2>
		<AccountsCard />
	</section>
</template>

<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'

import AccountsCard from './accounts-card.vue'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	minecraftAccount: {
		id: 'minecraft-account.label',
		defaultMessage: 'Minecraft account',
	},
})
</script>
