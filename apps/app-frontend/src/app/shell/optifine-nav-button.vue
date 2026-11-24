<template>
	<NavButton
		v-tooltip:[tooltipSide]="formatMessage(optifineMessages.navigation)"
		to="/optifine"
		:is-primary="(route) => route.path === '/optifine'"
	>
		<OptifineIcon class="size-6" />
	</NavButton>
</template>

<script setup lang="ts">
import { useVIntl } from '@modrinth/ui'

import OptifineIcon from '@/assets/optifine.svg'
import NavButton from '@/components/ui/NavButton.vue'
import { optifineMessages } from '@/features/optifine/messages'

withDefaults(
	defineProps<{
		tooltipSide?: string
	}>(),
	{
		tooltipSide: 'right',
	},
)

const { formatMessage } = useVIntl()
</script>
