package com.escudo.vpn.ui.navigation

import androidx.compose.runtime.Composable
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import com.escudo.vpn.ui.screens.AccountScreen
import com.escudo.vpn.ui.screens.FamilyScreen
import com.escudo.vpn.ui.screens.HomeScreen
import com.escudo.vpn.ui.screens.LoginScreen
import com.escudo.vpn.ui.screens.ProtectionScreen
import com.escudo.vpn.ui.screens.ResidentialRoutesScreen
import com.escudo.vpn.ui.screens.ServersScreen

object Routes {
    const val LOGIN = "login"
    const val HOME = "home"
    const val SERVERS = "servers"
    const val RESIDENTIAL = "residential"
    const val PROTECTION = "protection"
    const val FAMILY = "family"
    const val ACCOUNT = "account"
}

@Composable
fun EscudoNavGraph(
    navController: NavHostController,
    startDestination: String
) {
    NavHost(
        navController = navController,
        startDestination = startDestination
    ) {
        composable(Routes.LOGIN) {
            LoginScreen(
                onLoginSuccess = {
                    navController.navigate(Routes.HOME) {
                        popUpTo(Routes.LOGIN) { inclusive = true }
                    }
                }
            )
        }

        composable(Routes.HOME) {
            HomeScreen(
                onOpenServers = { navController.navigate(Routes.SERVERS) }
            )
        }

        composable(Routes.SERVERS) {
            ServersScreen(
                onServerSelected = { navController.popBackStack() }
            )
        }

        composable(Routes.RESIDENTIAL) {
            ResidentialRoutesScreen(
                onRouteSelected = { navController.navigate(Routes.HOME) }
            )
        }

        composable(Routes.PROTECTION) {
            ProtectionScreen()
        }

        composable(Routes.FAMILY) {
            FamilyScreen()
        }

        composable(Routes.ACCOUNT) {
            AccountScreen(
                onLogout = {
                    navController.navigate(Routes.LOGIN) {
                        popUpTo(0) { inclusive = true }
                    }
                }
            )
        }
    }
}
