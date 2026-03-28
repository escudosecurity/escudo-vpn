package com.escudo.vpn.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ChildCare
import androidx.compose.material.icons.filled.Event
import androidx.compose.material.icons.filled.Link
import androidx.compose.material.icons.filled.Schedule
import androidx.compose.material.icons.filled.Shield
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.escudo.vpn.data.model.DevicePolicyResponse
import com.escudo.vpn.data.model.FamilyOverview
import com.escudo.vpn.data.model.ParentalChild
import com.escudo.vpn.data.model.ParentalEvent
import com.escudo.vpn.data.repository.FamilyRepository
import com.escudo.vpn.ui.theme.Accent
import com.escudo.vpn.ui.theme.Background
import com.escudo.vpn.ui.theme.CardBackground
import com.escudo.vpn.ui.theme.TextSecondary
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

@HiltViewModel
class FamilyViewModel @Inject constructor(
    private val familyRepository: FamilyRepository
) : ViewModel() {
    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _overview = MutableStateFlow(FamilyOverview())
    val overview: StateFlow<FamilyOverview> = _overview.asStateFlow()

    private val _devicePolicy = MutableStateFlow(DevicePolicyResponse())
    val devicePolicy: StateFlow<DevicePolicyResponse> = _devicePolicy.asStateFlow()

    init {
        refresh()
    }

    fun refresh() {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null

            familyRepository.getFamilyOverview()
                .onSuccess { _overview.value = it }
                .onFailure { _error.value = it.message ?: "Falha ao carregar visão parental" }

            familyRepository.getDevicePolicy()
                .onSuccess { _devicePolicy.value = it }
                .onFailure {
                    if (_error.value == null) {
                        _error.value = it.message ?: "Falha ao carregar política do dispositivo"
                    }
                }

            _isLoading.value = false
        }
    }
}

@Composable
fun FamilyScreen(
    viewModel: FamilyViewModel = hiltViewModel()
) {
    val isLoading by viewModel.isLoading.collectAsState()
    val error by viewModel.error.collectAsState()
    val overview by viewModel.overview.collectAsState()
    val devicePolicy by viewModel.devicePolicy.collectAsState()
    var expandedChildId by remember { mutableStateOf<String?>(null) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Background)
            .padding(20.dp)
            .verticalScroll(rememberScrollState())
    ) {
        Text(
            text = "Família",
            style = MaterialTheme.typography.headlineMedium
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "Perfis infantis, janelas de uso e eventos ao vivo do dispositivo supervisionado.",
            style = MaterialTheme.typography.bodyMedium,
            color = TextSecondary
        )

        Spacer(modifier = Modifier.height(24.dp))

        if (isLoading) {
            CircularProgressIndicator(color = Accent)
            Spacer(modifier = Modifier.height(16.dp))
        }

        if (error != null) {
            Text(
                text = error!!,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodyMedium
            )
            Spacer(modifier = Modifier.height(16.dp))
        }

        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = androidx.compose.foundation.shape.RoundedCornerShape(18.dp),
            colors = CardDefaults.cardColors(containerColor = CardBackground)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("Resumo parental", style = MaterialTheme.typography.titleLarge)
                Spacer(modifier = Modifier.height(12.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceEvenly
                ) {
                    FamilyStat("Perfis", overview.totalChildren.toString())
                    FamilyStat("Vinculados", overview.linkedChildren.toString())
                    FamilyStat("Dispositivos", overview.activeChildDevices.toString())
                }
                Spacer(modifier = Modifier.height(12.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceEvenly
                ) {
                    FamilyStat("Políticas", overview.activePolicies.toString())
                    FamilyStat("Horários", overview.activeSchedules.toString())
                    FamilyStat("Eventos 24h", overview.recentEvents.toString())
                }
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        DevicePolicyCard(devicePolicy = devicePolicy)

        Spacer(modifier = Modifier.height(16.dp))

        overview.children.forEach { child ->
            ChildCard(
                child = child,
                expanded = expandedChildId == child.id,
                onToggle = {
                    expandedChildId = if (expandedChildId == child.id) null else child.id
                }
            )
            Spacer(modifier = Modifier.height(12.dp))
        }
    }
}

@Composable
private fun DevicePolicyCard(devicePolicy: DevicePolicyResponse) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = androidx.compose.foundation.shape.RoundedCornerShape(18.dp),
        colors = CardDefaults.cardColors(containerColor = CardBackground)
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text("Este dispositivo", style = MaterialTheme.typography.titleLarge)
            Spacer(modifier = Modifier.height(12.dp))
            SummaryLine(Icons.Default.Link, "Install ID", devicePolicy.deviceInstallId ?: "não gerado")
            Spacer(modifier = Modifier.height(8.dp))
            SummaryLine(
                Icons.Default.ChildCare,
                "Perfil ativo",
                devicePolicy.child?.name ?: "nenhum perfil infantil vinculado"
            )
            Spacer(modifier = Modifier.height(8.dp))
            SummaryLine(
                Icons.Default.Shield,
                "Bloqueios ativos",
                if (devicePolicy.effectivePolicies.isEmpty()) "nenhum" else {
                    buildList {
                        if (devicePolicy.effectivePolicies.any { it.blockTiktok }) add("TikTok")
                        if (devicePolicy.effectivePolicies.any { it.blockYoutube }) add("YouTube")
                        if (devicePolicy.effectivePolicies.any { it.blockSocialMedia }) add("social")
                        if (devicePolicy.effectivePolicies.any { it.blockStreaming }) add("streaming")
                    }.joinToString(", ").ifBlank { "personalizados" }
                }
            )
            Spacer(modifier = Modifier.height(8.dp))
            SummaryLine(
                Icons.Default.Schedule,
                "Horários ativos",
                devicePolicy.effectiveSchedules.joinToString(", ") { it.name }.ifBlank { "nenhum" }
            )
            if (devicePolicy.recentEvents.isNotEmpty()) {
                Spacer(modifier = Modifier.height(12.dp))
                Text("Últimos eventos", style = MaterialTheme.typography.titleMedium)
                Spacer(modifier = Modifier.height(8.dp))
                devicePolicy.recentEvents.take(5).forEach { event ->
                    EventRow(event)
                    Spacer(modifier = Modifier.height(6.dp))
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ChildCard(
    child: ParentalChild,
    expanded: Boolean,
    onToggle: () -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = androidx.compose.foundation.shape.RoundedCornerShape(18.dp),
        colors = CardDefaults.cardColors(containerColor = CardBackground),
        onClick = onToggle
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(child.name, style = MaterialTheme.typography.titleLarge)
            Spacer(modifier = Modifier.height(6.dp))
            Text(
                text = "Código ${child.accessCode} • ${child.tier}",
                color = Accent,
                style = MaterialTheme.typography.bodyMedium
            )
            Spacer(modifier = Modifier.height(10.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                FamilyStat("Dispositivos", child.devices.count { it.isActive }.toString())
                FamilyStat("Políticas", child.policies.size.toString())
                FamilyStat("Horários", child.schedules.size.toString())
            }

            if (expanded) {
                Spacer(modifier = Modifier.height(14.dp))
                child.devices.forEach { device ->
                    SummaryLine(
                        Icons.Default.Link,
                        device.displayName,
                        listOfNotNull(device.platform, device.deviceInstallId).joinToString(" • ")
                            .ifBlank { "dispositivo supervisionado" }
                    )
                    Spacer(modifier = Modifier.height(6.dp))
                }
                child.policies.forEach { policy ->
                    SummaryLine(
                        Icons.Default.Shield,
                        "Política",
                        buildList {
                            if (policy.blockTiktok) add("TikTok")
                            if (policy.blockYoutube) add("YouTube")
                            if (policy.blockSocialMedia) add("social")
                            if (policy.blockStreaming) add("streaming")
                            if (policy.maxDailyMinutes != null) add("${policy.maxDailyMinutes} min/dia")
                        }.joinToString(", ").ifBlank { "personalizada" }
                    )
                    Spacer(modifier = Modifier.height(6.dp))
                }
                child.schedules.forEach { schedule ->
                    SummaryLine(
                        Icons.Default.Schedule,
                        schedule.name,
                        "${schedule.startMinute}–${schedule.endMinute} • ${schedule.blockedCategories.joinToString(", ").ifBlank { "regras de app" }}"
                    )
                    Spacer(modifier = Modifier.height(6.dp))
                }
            }
        }
    }
}

@Composable
private fun SummaryLine(icon: ImageVector, title: String, value: String) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Icon(icon, contentDescription = null, tint = Accent)
        Spacer(modifier = Modifier.width(10.dp))
        Column {
            Text(title, style = MaterialTheme.typography.titleSmall)
            Text(value, style = MaterialTheme.typography.bodyMedium, color = TextSecondary)
        }
    }
}

@Composable
private fun FamilyStat(label: String, value: String) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(value, style = MaterialTheme.typography.titleLarge, color = Accent)
        Text(label, style = MaterialTheme.typography.bodySmall, color = TextSecondary)
    }
}

@Composable
private fun EventRow(event: ParentalEvent) {
    SummaryLine(
        icon = Icons.Default.Event,
        title = event.eventType.replace('_', ' '),
        value = listOfNotNull(event.appIdentifier, event.domain, event.detail).joinToString(" • ")
            .ifBlank { event.occurredAt }
    )
}
