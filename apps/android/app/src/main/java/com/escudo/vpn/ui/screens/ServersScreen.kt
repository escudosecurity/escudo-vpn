package com.escudo.vpn.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.escudo.vpn.data.model.Server
import com.escudo.vpn.data.repository.VpnRepository
import com.escudo.vpn.ui.components.ServerCard
import com.escudo.vpn.ui.model.ServerCategory
import com.escudo.vpn.ui.model.toPresentation
import com.escudo.vpn.ui.theme.Accent
import com.escudo.vpn.ui.theme.Background
import com.escudo.vpn.ui.theme.CardBackground
import com.escudo.vpn.ui.theme.TextSecondary
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

@HiltViewModel
class ServersViewModel @Inject constructor(
    private val vpnRepository: VpnRepository
) : ViewModel() {

    private val _servers = MutableStateFlow<List<Server>>(emptyList())
    val servers: StateFlow<List<Server>> = _servers.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _selectedServerId = MutableStateFlow<String?>(null)
    val selectedServerId: StateFlow<String?> = _selectedServerId.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    init {
        _selectedServerId.value = vpnRepository.getSelectedServerId()
        loadServers()
    }

    fun loadServers() {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            val result = vpnRepository.getServers()
            _isLoading.value = false
            result.fold(
                onSuccess = { servers ->
                    _servers.value = servers
                    if (servers.size == 1 && _selectedServerId.value == null) {
                        selectServer(servers[0])
                    }
                },
                onFailure = { _error.value = it.message ?: "Erro ao carregar servidores" }
            )
        }
    }

    fun selectServer(server: Server) {
        _selectedServerId.value = server.id
        vpnRepository.saveSelectedServer(server.id, server.name)
    }
}

@Composable
fun ServersScreen(
    onServerSelected: () -> Unit,
    viewModel: ServersViewModel = hiltViewModel()
) {
    val servers by viewModel.servers.collectAsState()
    val isLoading by viewModel.isLoading.collectAsState()
    val selectedServerId by viewModel.selectedServerId.collectAsState()
    val error by viewModel.error.collectAsState()

    val standardServers = servers.filter { it.toPresentation().category == ServerCategory.STANDARD }
    val groupedServers = standardServers
        .groupBy { it.serviceClass ?: "Free" }
        .toList()
        .sortedBy { (serviceClass, _) ->
            when (serviceClass) {
                "Free" -> 0
                "Medium" -> 1
                "Power" -> 2
                else -> 3
            }
        }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Background)
    ) {
        Column(modifier = Modifier.fillMaxSize()) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(start = 20.dp, top = 20.dp, end = 20.dp, bottom = 12.dp)
            ) {
                Text(
                    text = "Rede global",
                    style = MaterialTheme.typography.headlineMedium
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Choose the normal global servers here. Residential routes stay in their own tab.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary
                )
                Spacer(modifier = Modifier.height(16.dp))
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .background(
                            color = CardBackground,
                            shape = androidx.compose.foundation.shape.RoundedCornerShape(18.dp)
                        )
                        .padding(horizontal = 16.dp, vertical = 14.dp),
                    horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        FleetFact("Free", standardServers.count { it.serviceClass == null || it.serviceClass == "Free" }.toString())
                        FleetFact("Medium", standardServers.count { it.serviceClass == "Medium" }.toString())
                        FleetFact("Power", standardServers.count { it.serviceClass == "Power" }.toString())
                    }
                }

            if (isLoading) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator(
                        color = Accent,
                        modifier = Modifier.size(48.dp)
                    )
                }
            } else if (error != null) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Text(
                            text = error!!,
                            color = MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.bodyMedium
                        )
                        Spacer(modifier = Modifier.height(16.dp))
                        Text(
                            text = "Toque para tentar novamente",
                            color = Accent,
                            style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier
                                .padding(8.dp)
                                .background(CardBackground, androidx.compose.foundation.shape.RoundedCornerShape(12.dp))
                                .padding(horizontal = 14.dp, vertical = 10.dp)
                        )
                    }
                }
            } else {
                LazyColumn(
                    contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    groupedServers.forEach { (serviceClass, itemsInSection) ->
                        item(key = serviceClass) {
                            Column(
                                modifier = Modifier.padding(top = 8.dp, bottom = 4.dp)
                            ) {
                                Text(
                                    text = serviceClass,
                                    style = MaterialTheme.typography.titleLarge
                                )
                            }
                        }
                        items(itemsInSection, key = { it.id }) { server ->
                            ServerCard(
                                server = server,
                                isSelected = server.id == selectedServerId,
                                onClick = {
                                    viewModel.selectServer(server)
                                    onServerSelected()
                                }
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun FleetFact(
    label: String,
    value: String
) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            text = value,
            style = MaterialTheme.typography.titleLarge,
            color = Accent
        )
        Text(
            text = label,
            style = MaterialTheme.typography.labelMedium,
            color = TextSecondary
        )
    }
}
