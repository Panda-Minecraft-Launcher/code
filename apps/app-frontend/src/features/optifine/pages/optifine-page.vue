<template>
	<div class="box-border h-full p-6">
		<div class="flex flex-col gap-4">
			<div class="flex flex-wrap items-center justify-between gap-3">
				<div class="flex flex-col gap-1">
					<h1 class="m-0 text-2xl font-semibold text-contrast">
						{{ formatMessage(optifineMessages.title) }}
					</h1>
					<p class="m-0 text-sm text-secondary">
						{{ formatMessage(optifineMessages.source) }}
					</p>
				</div>
				<Button type="colored" color="brand" @click="installModal?.show()">
					<PlusIcon />
					{{ formatMessage(optifineMessages.install) }}
				</Button>
			</div>
			<LibrarySection :instances="instances" :show-new-instance="false" />
		</div>
		<OptifineInstallModal ref="installModal" />
	</div>
</template>

<script setup lang="ts">
import { PlusIcon } from '@modrinth/assets'
import { Button, useVIntl } from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed, useTemplateRef } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import OptifineIcon from '@/assets/optifine.svg'
import LibrarySection from '@/components/ui/library/index.vue'
import { optifineMessages } from '@/features/optifine/messages'
import OptifineInstallModal from '@/features/optifine/optifine-install-modal.vue'
import { optifineInstancesQueryOptions } from '@/features/optifine/queries'
import { useRootBreadcrumb } from '@/shared/breadcrumbs'

defineOptions({ name: 'OptifinePage' })

const { formatMessage } = useVIntl()
const installModal = useTemplateRef<ComponentExposed<typeof OptifineInstallModal>>('installModal')

const instancesQuery = useQuery(optifineInstancesQueryOptions())
const instances = computed(() => instancesQuery.data.value ?? [])

useRootBreadcrumb({
	slot: 'root',
	id: 'optifine',
	label: formatMessage(optifineMessages.title),
	to: '/optifine',
	visual: { type: 'icon', component: OptifineIcon },
})
</script>
