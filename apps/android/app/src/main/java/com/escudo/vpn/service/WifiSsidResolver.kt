package com.escudo.vpn.service

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.net.ConnectivityManager
import android.net.Network
import android.net.wifi.WifiInfo
import android.net.wifi.WifiManager
import android.os.Build
import androidx.core.content.ContextCompat

object WifiSsidResolver {
    fun getCurrentSsid(context: Context, network: Network? = null): String? {
        if (!hasRequiredPermission(context)) return null

        val connectivityManager = context.getSystemService(ConnectivityManager::class.java)
        val wifiInfo = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            connectivityManager?.getNetworkCapabilities(network ?: connectivityManager.activeNetwork)
                ?.transportInfo as? WifiInfo
        } else {
            @Suppress("DEPRECATION")
            context.applicationContext.getSystemService(Context.WIFI_SERVICE)
                ?.let { it as WifiManager }
                ?.connectionInfo
        }

        val ssid = wifiInfo?.ssid ?: return null
        return ssid.removePrefix("\"").removeSuffix("\"").takeIf {
            it.isNotBlank() && it != WifiManager.UNKNOWN_SSID && it != "<unknown ssid>"
        }
    }

    fun hasRequiredPermission(context: Context): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return true

        return ContextCompat.checkSelfPermission(
            context,
            Manifest.permission.ACCESS_FINE_LOCATION
        ) == PackageManager.PERMISSION_GRANTED
    }
}
