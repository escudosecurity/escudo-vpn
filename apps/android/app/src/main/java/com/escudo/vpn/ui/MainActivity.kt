package com.escudo.vpn.ui

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AccountCircle
import androidx.compose.material.icons.filled.ChildCare
import androidx.compose.material.icons.filled.Public
import androidx.compose.material.icons.filled.Security
import androidx.compose.material.icons.filled.Shield
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.navigation.NavDestination.Companion.hierarchy
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.escudo.vpn.data.prefs.SecurePrefs
import com.escudo.vpn.ui.navigation.EscudoNavGraph
import com.escudo.vpn.ui.navigation.Routes
import com.escudo.vpn.ui.theme.Accent
import com.escudo.vpn.ui.theme.Background
import com.escudo.vpn.ui.theme.CardBackground
import com.escudo.vpn.ui.theme.EscudoTheme
import com.escudo.vpn.ui.theme.TextSecondary
import dagger.hilt.android.AndroidEntryPoint
import javax.inject.Inject

@AndroidEntryPoint
class MainActivity : ComponentActivity() {

    companion object {
        const val AUDIT_ACTION_CONNECT = "connect"
        const val AUDIT_ACTION_DISCONNECT = "disconnect"
    }

    @Inject
    lateinit var securePrefs: SecurePrefs

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        setContent {
            EscudoTheme {
                EscudoMainScreen(isLoggedIn = securePrefs.isLoggedIn())
            }
        }
    }
}

@Composable
fun EscudoMainScreen(isLoggedIn: Boolean) {
    val navController = rememberNavController()
    val startDestination = if (isLoggedIn) Routes.HOME else Routes.LOGIN

    val bottomNavItems = listOf(
        BottomNavItem(Routes.HOME, "VPN", Icons.Default.Shield),
        BottomNavItem(Routes.RESIDENTIAL, "Residential", Icons.Default.Public),
        BottomNavItem(Routes.PROTECTION, "Proteção", Icons.Default.Security),
        BottomNavItem(Routes.FAMILY, "Família", Icons.Default.ChildCare),
        BottomNavItem(Routes.ACCOUNT, "Conta", Icons.Default.AccountCircle)
    )

    val navBackStackEntry by navController.currentBackStackEntryAsState()
    val currentDestination = navBackStackEntry?.destination
    val showBottomBar = currentDestination?.hierarchy?.any { dest ->
        bottomNavItems.any { it.route == dest.route }
    } == true

    Scaffold(
        containerColor = Background,
        bottomBar = {
            if (showBottomBar) {
                NavigationBar(
                    containerColor = CardBackground,
                    contentColor = Accent
                ) {
                    bottomNavItems.forEach { item ->
                        val isSelected = currentDestination?.hierarchy?.any {
                            it.route == item.route
                        } == true

                        NavigationBarItem(
                            selected = isSelected,
                            onClick = {
                                navController.navigate(item.route) {
                                    popUpTo(navController.graph.findStartDestination().id) {
                                        saveState = true
                                    }
                                    launchSingleTop = true
                                    restoreState = true
                                }
                            },
                            icon = {
                                Icon(
                                    imageVector = item.icon,
                                    contentDescription = item.label
                                )
                            },
                            label = { Text(text = item.label) },
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = Accent,
                                selectedTextColor = Accent,
                                unselectedIconColor = TextSecondary,
                                unselectedTextColor = TextSecondary,
                                indicatorColor = CardBackground
                            )
                        )
                    }
                }
            }
        }
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .padding(innerPadding)
                .background(Background)
        ) {
            EscudoNavGraph(
                navController = navController,
                startDestination = startDestination
            )
        }
    }
}
