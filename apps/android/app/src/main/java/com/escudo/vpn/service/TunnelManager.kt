package com.escudo.vpn.service

import com.wireguard.config.Config
import com.wireguard.config.InetEndpoint
import com.wireguard.config.InetNetwork
import com.wireguard.config.Interface
import com.wireguard.config.Peer
import com.wireguard.crypto.Key
import java.io.BufferedReader
import java.io.StringReader
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class TunnelManager @Inject constructor() {

    private var currentConfig: Config? = null

    fun parseConfig(configString: String): Config {
        val reader = BufferedReader(StringReader(configString))
        val config = Config.parse(reader)
        currentConfig = config
        return config
    }

    fun getCurrentConfig(): Config? {
        return currentConfig
    }

    fun clearConfig() {
        currentConfig = null
    }
}
