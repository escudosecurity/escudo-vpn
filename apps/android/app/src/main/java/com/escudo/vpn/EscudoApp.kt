package com.escudo.vpn

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Intent
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.util.Log
import com.escudo.vpn.data.prefs.SecurePrefs
import com.escudo.vpn.service.EscudoVpnService
import com.escudo.vpn.service.WifiSsidResolver
import dagger.hilt.android.HiltAndroidApp

@HiltAndroidApp
class EscudoApp : Application() {

    companion object {
        const val VPN_CHANNEL_ID = "escudo_vpn_channel"
    }

    private var networkCallback: ConnectivityManager.NetworkCallback? = null

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        registerWifiMonitor()
    }

    private fun registerWifiMonitor() {
        val prefs = SecurePrefs(this)
        if (!prefs.isAutoProtectEnabled()) return

        val connectivityManager = getSystemService(ConnectivityManager::class.java) ?: return
        val request = NetworkRequest.Builder()
            .addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
            .build()

        networkCallback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                if (!prefs.isLoggedIn()) return
                if (android.net.VpnService.prepare(this@EscudoApp) != null) return
                if (!WifiSsidResolver.hasRequiredPermission(this@EscudoApp)) {
                    Log.i("EscudoWifi", "Auto-protect enabled but location permission is missing")
                    return
                }

                val ssid = WifiSsidResolver.getCurrentSsid(this@EscudoApp, network) ?: return

                val trustedNetworks = prefs.getTrustedNetworks()
                if (!trustedNetworks.contains(ssid)) {
                    Log.i("EscudoWifi", "Untrusted WiFi detected, auto-connecting VPN")
                    val intent = Intent(this@EscudoApp, EscudoVpnService::class.java)
                    intent.putExtra(EscudoVpnService.EXTRA_AUTO_CONNECT, true)
                    androidx.core.content.ContextCompat.startForegroundService(this@EscudoApp, intent)
                }
            }
        }

        connectivityManager.registerNetworkCallback(request, networkCallback!!)
    }

    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            VPN_CHANNEL_ID,
            getString(R.string.vpn_channel_name),
            NotificationManager.IMPORTANCE_LOW
        ).apply {
            description = getString(R.string.vpn_channel_description)
            setShowBadge(false)
        }
        val notificationManager = getSystemService(NotificationManager::class.java)
        notificationManager.createNotificationChannel(channel)
    }
}
