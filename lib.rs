#![cfg_attr(not(feature = "std"), no_std, no_main)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[warn(clippy::arithmetic_side_effects)]

#[ink::contract]
pub mod TrabajoFinal {
    use scale_info::prelude::format;
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    use ink::prelude::string::ToString;

    enum ERRORES
    {
        NO_ES_ADMINISTRADOR,
        USUARIO_NO_REGISTRADO,
    }

    #[derive(scale::Decode, scale::Encode, Debug)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout,PartialEq,Clone))]
    pub enum ESTADO_ELECCION
    {
        CERRADA,
        ABIERTA,
        INICIADA,
        FINALIZADA
    }

    #[derive(scale::Decode, scale::Encode, Debug)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout,PartialEq,Clone))]
    pub enum TIPO_DE_USUARIO
    {
        VOTANTE,
        CANDIDATO
    }

    impl ERRORES
    {
        fn to_string(&self) -> String
        {
            match self 
            {
                ERRORES::NO_ES_ADMINISTRADOR => String::from("No eres el administrador."),
                ERRORES::USUARIO_NO_REGISTRADO => String::from("No estás registrado en el sistema. Espera a que te acepten en el mismo o realiza la solicitud.")
            }
        }
    }

    #[derive(scale::Decode, scale::Encode, Debug)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    struct Usuario
    {
        id:AccountId,
        nombre:String,
        apellido:String,
        dni:String,
    }

    #[derive(scale::Decode, scale::Encode, Debug)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    struct Votante
    {
        usuario_id:AccountId,
        voto_emitido:bool,
    }

    #[derive(scale::Decode, scale::Encode, Debug)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    struct CandidatoConteo
    {
        id:u64,
        usuario_id:AccountId,
        votos_totales:u64,
    }

    #[derive(scale::Decode, scale::Encode, Debug)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    struct Eleccion
    {
        id:u64,
        candidatos:Vec<CandidatoConteo>,
        votantes:Vec<Votante>,
        usuarios_rechazados:Vec<AccountId>,
        usuarios_pendientes:Vec<(AccountId,TIPO_DE_USUARIO)>,
        estado: ESTADO_ELECCION, 
        fecha_inicio:u64,
        fecha_final:u64,
    }

    impl Eleccion
    {
        fn inscripcion_abierta(&self) -> bool {
            match self.estado {
                ESTADO_ELECCION::ABIERTA => true,
                _ => false
            }
        }
        fn votacion_abierta(&self) -> bool {
            match self.estado {
                ESTADO_ELECCION::INICIADA => true,
                _ => false
            }
        }
        fn esta_cerrada(&self) -> bool {
            match self.estado {
                ESTADO_ELECCION::CERRADA => true,
                _ => false
            }
        }
        fn esta_finalizada(&self) -> bool {
            match self.estado {
                ESTADO_ELECCION::FINALIZADA => true,
                _ => false
            }
        }

        fn es_votante(&self, id: AccountId) -> bool {
            self.votantes.iter().any(|vot| vot.usuario_id == id)
        }
        fn es_candidato(&self, id: AccountId) -> bool {
            self.candidatos.iter().any(|cand| cand.usuario_id == id)
        }

        fn es_usuario_pendiente(&self, id: AccountId) -> bool {
            self.usuarios_pendientes.iter().any(|(usuario_id, _tipo)| *usuario_id == id)
        }
        fn es_usuario_rechazado(&self, id: AccountId) -> bool {
            self.usuarios_rechazados.iter().any(|uids| *uids == id)
        }

        fn existe_candidato(&self, candidato_id:u64) -> bool
        {
            candidato_id >= 1 && candidato_id <= self.candidatos.len() as u64
        }

        fn obtener_informacion_candidato(&self, candidato_id:u64) -> Option<&CandidatoConteo> //Quizás se debería de cambiar a un campo específico del candidato como por ejemplo discurso/ideas, y también su nombre
        {
            if !self.existe_candidato(candidato_id) { return None; }
            match (candidato_id as usize).checked_sub(1) {
                None => None,
                Some(index) => Some(&self.candidatos[index])
            }
        }

        pub fn votar_candidato(&mut self, votante_id:AccountId, candidato_id:u64) -> Result<String, String>
        {
            if !self.existe_candidato(candidato_id) { return Err(String::from("No existe un candidato con este id.")); }

            let votante = match self.votantes.iter_mut().find(|votante| votante.usuario_id == votante_id) {
                Some(votante) => votante,
                None => return Err(String::from("No estás registrado en la elección."))
            };
            if votante.voto_emitido { return Err(String::from("No se realizó el voto porque ya votaste anteriormente.")); }
            
            let candidato = match (candidato_id as usize).checked_sub(1) {
                None => return Err(String::from("Se produjo un overflow intentando obtener el candidato.")),
                Some(index) => &mut self.candidatos[index]
            };

            match candidato.votos_totales.checked_add(1) {
                None => {
                    return Err(String::from("Se produjo un overflow al intentar sumar el voto."));
                },
                Some(nuevo_votos_totales) => { 
                    votante.voto_emitido = true;
                    candidato.votos_totales = nuevo_votos_totales;
                    return Ok(String::from("Voto emitido exitosamente."));
                }
            }
        }

        ///Usado por el administrador.
        ///Revisa el primer usuario pendiente.
        ///Lo envia al Vec candidato si es candidato, o votante en caso contrario.
        pub fn procesar_siguiente_usuario_pendiente(&mut self, aceptar_usuario:bool) -> Result<String, String>
        {
            let sig_usuario = self.usuarios_pendientes.first();
            if sig_usuario.is_none() { return Err(String::from("No hay usuarios pendientes.")); }

            let (usuario, tipo) = self.usuarios_pendientes.remove(0);
            if aceptar_usuario { 
                match tipo {
                    TIPO_DE_USUARIO::VOTANTE =>{
                    self.votantes.push(Votante{
                        usuario_id:usuario,
                        voto_emitido:false,
                    });

                   },
                   TIPO_DE_USUARIO::CANDIDATO=>{
                    let candidato_id_check = (self.candidatos.len() as u64).checked_add(1);
                    let candidato_id:u64;
                    match candidato_id_check {
                        Some(id_validado) => candidato_id = id_validado,
                        None => return Err(String::from("Ocurrio un overflow al calcular la ID del candidato.")),
                    }
                    self.candidatos.push(CandidatoConteo{
                        id: candidato_id,
                        usuario_id: usuario,
                        votos_totales: 0,
                    });
                   },
                }
                Ok(String::from("Usuario agregado exitosamente."))
            }
            else{
                self.usuarios_rechazados.push(usuario);
                Ok(String::from("Usuario rechazado exitosamente."))
            }
        }
    }

    #[ink(storage)]
    pub struct TrabajoFinal {
        administrador:AccountId,
        registro_activado:bool,
        usuarios:Vec<Usuario>,
        usuarios_pendientes:Vec<Usuario>,
        usuarios_rechazados:Vec<Usuario>,
        elecciones:Vec<Eleccion>,
    }

    impl TrabajoFinal {

        #[ink(constructor)]
        pub fn new() -> Self {
            Self { 
                administrador: Self::env().caller(),
                registro_activado: false,
                usuarios: Vec::new(),
                usuarios_pendientes: Vec::new(),
                usuarios_rechazados: Vec::new(),
                elecciones: Vec::new(),
            }
        }

        fn es_administrador(&self) -> bool {
            self.env().caller() == self.administrador
        }

        fn es_usuario_registrado(&self, id: AccountId) -> bool {
            self.usuarios.iter().any(|usuario| usuario.id == id)
        }
        fn es_usuario_pendiente(&self, id: AccountId) -> bool {
            self.usuarios_pendientes.iter().any(|usuario| usuario.id == id)
        }
        fn es_usuario_rechazado(&self, id: AccountId) -> bool {
            self.usuarios_rechazados.iter().any(|usuario| usuario.id == id)
        }

        fn obtener_usuario_por_id(&mut self, id_usuario: AccountId) -> Option<&Usuario> {
            if self.es_usuario_registrado(id_usuario) { 
                    return self.usuarios.iter().find(|user| user.id == id_usuario);
            }
            None
        }
        fn obtener_usuario_pendiente_por_id(&mut self, id_usuario: AccountId) -> Option<&Usuario> {
            if self.es_usuario_pendiente(id_usuario) { 
                return self.usuarios_pendientes.iter().find(|user| user.id == id_usuario);
            }
            None
        }
        fn obtener_usuario_rechazado_por_id(&mut self, id_usuario: AccountId) -> Option<&Usuario> {
            if self.es_usuario_rechazado(id_usuario) { 
                return self.usuarios_rechazados.iter().find(|user| user.id == id_usuario);
            }
            None
        }

        fn existe_eleccion(&self, eleccion_id:u64) -> bool
        {
            if eleccion_id >= 1 && eleccion_id <= self.elecciones.len() as u64 {
                return true;
            }
            false
        }

        fn obtener_eleccion_por_id(&mut self, eleccion_id:u64) -> Option<&mut Eleccion> {
            if self.existe_eleccion(eleccion_id) {
                let index = eleccion_id.checked_sub(1);
                match index {
                    Some(index_valid) => {
                        return Some(&mut self.elecciones[index_valid as usize])
                        }
                    None => {
                        return None
                    }
                }
            }
            return None;
        }

        fn validar_estado_eleccion_para_inscripciones(&mut self,eleccion_id:u64, block_timestamp:u64) -> Result<&mut Eleccion,String>{
            let option_eleccion = self.obtener_eleccion_por_id(eleccion_id);
            if option_eleccion.is_none() { 
                return Err(String::from("No existe una elección con ese id."));
            }
            
            let eleccion = option_eleccion.unwrap();
            
            if !eleccion.inscripcion_abierta() {
                return Err(String::from("La eleccion no esta abierta, no te puedes registrar."));
            }
            if eleccion.fecha_final < block_timestamp {
                if !eleccion.esta_finalizada() {
                    eleccion.estado = ESTADO_ELECCION::FINALIZADA;
                } 
                return Err(String::from("La elección ya finalizó, no te puedes registrar."));
            }
            if eleccion.fecha_inicio < block_timestamp {
                if !eleccion.votacion_abierta() {
                    eleccion.estado = ESTADO_ELECCION::INICIADA;
                }
                return Err(String::from("La votación en la elección ya comenzó, no te puedes registrar."));
            }
            Ok(eleccion)
        }
        fn validar_estado_eleccion_para_votaciones(&mut self,eleccion_id:u64, block_timestamp:u64) -> Result<&mut Eleccion,String>{
            let option_eleccion = self.obtener_eleccion_por_id(eleccion_id);
            if option_eleccion.is_none() { 
                return Err(String::from("No existe una elección con ese id."));
            }
            
            let eleccion = option_eleccion.unwrap();
            
            if eleccion.fecha_final < block_timestamp {
                if !eleccion.esta_finalizada() {
                    eleccion.estado = ESTADO_ELECCION::FINALIZADA;
                } 
                return Err(String::from("La elección ya finalizó, no puedes votar."));
            }
            if eleccion.fecha_inicio > block_timestamp {
                return Err(String::from("La eleccion no inicio, no puedes votar."));
            }

            if eleccion.fecha_inicio < block_timestamp && !eleccion.votacion_abierta() {
                eleccion.estado = ESTADO_ELECCION::INICIADA;
            }
            Ok(eleccion)
        }


    // ===================================================================================================
    // =========================creacion y administracion de usuarios=====================================
    // ===================================================================================================

        /// Utilizado por los usuarios para poder registrarse en el sistema.
        /// Luego de registrarse queda pendiente de aceptación por parte de un Administrador.
        /// Si tu registro es rechazado, no podrás volver a intentar registrarte.
        #[ink(message)] //FUNCIONA
        pub fn registrarse(&mut self, nombre:String, apellido:String, dni:String) -> Result<String, String>
        {
            if !self.registro_activado { return Err(String::from("El registro todavía no está activado.")); }
            if self.es_administrador() { return Err(String::from("Eres el administrador, no puedes registrarte.")); }

            let id = self.env().caller();
            if self.es_usuario_rechazado(id) {  return Err(String::from("Tu solicitud de registro ya fue rechazada.")); }
            if self.es_usuario_registrado(id) { return Err(String::from("Ya estás registrado como usuario.")); }
            if self.es_usuario_pendiente(id) { return Err(String::from("Ya estás en la cola de usuarios pendientes.")); }

            let usuario = Usuario { id, nombre, apellido, dni };
            self.usuarios_pendientes.push(usuario);
            return Ok(String::from("Registro exitoso. Se te añadió en la cola de usuarios pendientes."));
        }

        /// Utilizado por un Administrador.
        /// Obtiene la información del próximo usuario a registrarse.
        #[ink(message)] //FUNCIONA
        pub fn obtener_informacion_siguiente_usuario_pendiente(&self) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            let sig_usuario = self.usuarios_pendientes.first();
            match sig_usuario {
                Some(usuario) => {
                    let mut str = String::from("Nombre: ") + usuario.nombre.as_str();
                    str.push_str((String::from("\nApellido: ") + usuario.apellido.as_str()).as_str());
                    str.push_str((String::from("\nDNI: ") + usuario.apellido.as_str()).as_str());
                    Ok(str)
                },
                None => Err(String::from("No hay usuarios pendientes.")),
            }
        }

        /// Utilizado por un Administrador.
        /// Se procesará el próximo usuario pendiente.
        /// Para obtener la información del mismo, utilizar obtenerInformacionSiguienteUsuarioPendiente
        /// Si se acepta el usuario, podrá utilizar el sistema.
        /// Si se rechaza el usuario, este no podrá volver a intentar registrarse en el sistema.
        #[ink(message)] //FUNCIONA
        pub fn procesar_siguiente_usuario_pendiente(&mut self, aceptar_usuario:bool) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            let sig_usuario = self.usuarios_pendientes.first();
            if sig_usuario.is_none() { return Err(String::from("No hay usuarios pendientes.")); }

            let usuario = self.usuarios_pendientes.remove(0);
            if aceptar_usuario { 
                self.usuarios.push(usuario);
                return Ok(String::from("Usuario agregado exitosamente."));
            }

            self.usuarios_rechazados.push(usuario);
            return Ok(String::from("Usuario rechazado exitosamente."));
        }


       
        #[ink(message)] //FUNCIONA
        pub fn obtener_usuarios_pendientes(&self) -> Vec<AccountId>
        {
            self.usuarios_pendientes.iter().map(|usuario| usuario.id).collect()
        }
        #[ink(message)] //FUNCIONA
        pub fn obtener_usuarios(&self) -> Vec<AccountId>
        {
            self.usuarios.iter().map(|usuario| usuario.id).collect()
        }
        #[ink(message)] //FUNCIONA
        pub fn obtener_usuarios_rechazados(&self) -> Vec<AccountId>
        {
            self.usuarios_rechazados.iter().map(|usuario| usuario.id).collect()
        }

        #[ink(message)] //FUNCIONA
        pub fn obtener_datos_usuario_pendiente_por_id(&mut self, id_usuario: AccountId) -> Result<String, String> 
        {
            let sig_usuario = self.obtener_usuario_pendiente_por_id(id_usuario);
            match sig_usuario {
                Some(usuario) => {
                    let mut str = String::from("Nombre: ") + usuario.nombre.as_str();
                    str.push_str((String::from("\nApellido: ") + usuario.apellido.as_str()).as_str());
                    str.push_str((String::from("\nDNI: ") + usuario.apellido.as_str()).as_str());
                    Ok(str)
                },
                None => Err(String::from("No hay usuarios con tal id.")),
            }
        }
        #[ink(message)] //FUNCIONA
        pub fn obtener_datos_usuario_por_id(&mut self, id_usuario: AccountId) -> Result<String, String> 
        {
            let sig_usuario = self.obtener_usuario_por_id(id_usuario);
            match sig_usuario {
                Some(usuario) => {
                    let mut str = String::from("Nombre: ") + usuario.nombre.as_str();
                    str.push_str((String::from("\nApellido: ") + usuario.apellido.as_str()).as_str());
                    str.push_str((String::from("\nDNI: ") + usuario.apellido.as_str()).as_str());
                    Ok(str)
                },
                None => Err(String::from("No hay usuarios con tal id.")),
            }
        }
        #[ink(message)] //FUNCIONA
        pub fn obtener_datos_usuario_rechazado_por_id(&mut self, id_usuario: AccountId) -> Result<String, String> 
        {
            let sig_usuario = self.obtener_usuario_rechazado_por_id(id_usuario);
            match sig_usuario {
                Some(usuario) => {
                    let mut str = String::from("Nombre: ") + usuario.nombre.as_str();
                    str.push_str((String::from("\nApellido: ") + usuario.apellido.as_str()).as_str());
                    str.push_str((String::from("\nDNI: ") + usuario.apellido.as_str()).as_str());
                    Ok(str)
                },
                None => Err(String::from("No hay usuarios con tal id.")),
            }
        }


        /// Utilizado por un administrador.
        /// Activa el registro de usuarios si no está activo el registro.
        #[ink(message)] //FUNCIONA
        pub fn activar_registro(&mut self) -> Result<String, String> 
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            if self.registro_activado { return Err(String::from("El registro ya está activado.")); }
            self.registro_activado = true;
            return Ok(String::from("Se activó el registro para los usuarios."));
        }
        /// Utilizado por un administrador.
        /// Desactiva el registro de usuarios si no está activo el registro.
        #[ink(message)] //FUNCIONA
        pub fn desactivar_registro(&mut self) -> Result<String, String> 
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            if !self.registro_activado { return Err(String::from("El registro ya está desactivado.")); }
            self.registro_activado = false;
            return Ok(String::from("Se desactivó el registro para los usuarios."));
        }


    // ===================================================================================================
    // ===================================================================================================
    // ===================================================================================================


    // ===================================================================================================
    // =========================creacion y administracion de estados de elecciones========================
    // ===================================================================================================

        /// Utilizado por un administrador.
        /// Crea una elección colocando fecha de inicio y final.
        #[ink(message)] //FUNCIONA
        pub fn crear_eleccion(&mut self, fecha_inicial:String, fecha_final:String) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }

            let fecha_inicial_milisegundos = chrono::NaiveDateTime::parse_from_str(&fecha_inicial, "%d-%m-%Y %H:%M");
            if fecha_inicial_milisegundos.is_err() {
                return Err(String::from("Error en el formato de la fecha inicial. Formato: dd-mm-YYYY hh:mm"));
            }
            let fecha_final_milisegundos = chrono::NaiveDateTime::parse_from_str(&fecha_final, "%d-%m-%Y %H:%M");
            if fecha_final_milisegundos.is_err() {
                return Err(String::from("Error en el formato de la fecha final. Formato: dd-mm-YYYY hh:mm"));
            }

            let eleccion_id_check = (self.elecciones.len() as u64).checked_add(1);
            let eleccion_id:u64;
            match eleccion_id_check {
                Some(id_validado) => eleccion_id = id_validado,
                None => return Err(String::from("Ocurrio un overflow al calcular la ID de la eleccion.")),
            }
            let eleccion = Eleccion {
                id: eleccion_id,
                candidatos: Vec::new(),
                votantes: Vec::new(),
                usuarios_pendientes: Vec::new(),
                usuarios_rechazados: Vec::new(),
                estado: ESTADO_ELECCION::CERRADA,
                fecha_inicio: fecha_inicial_milisegundos.unwrap().and_utc().timestamp_millis() as u64,
                fecha_final: fecha_final_milisegundos.unwrap().and_utc().timestamp_millis() as u64,
            };
            self.elecciones.push(eleccion);

            return Ok(String::from("Eleccion creada exitosamente. Id de la elección: ") + &eleccion_id.to_string());
        }

        /// Utilizado por un administrador.
        /// cierra una elección colocando su estado en CERRADO (estado anterior al INICIADA).
        #[ink(message)] //FUNCIONA //MERJORAR
        pub fn cerrar_eleccion(&mut self, eleccion_id: u64) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }

            let block_timestamp = self.env().block_timestamp();
            let eleccion_option = self.obtener_eleccion_por_id(eleccion_id);
            match eleccion_option {
                Some(eleccion) => {
                    if eleccion.fecha_inicio < block_timestamp {
                        return Err(String::from("La eleccion se encuentra en la fecha de votacion!"));
                    }
                    match eleccion.estado {
                        ESTADO_ELECCION::CERRADA => Err(String::from("La eleccion ya se encuantra en el estado correspondiente!")),
                        ESTADO_ELECCION::INICIADA => Err(String::from("La eleccion se encuentra en el estado de votacion!")),
                        ESTADO_ELECCION::FINALIZADA => Err(String::from("La eleccion ya cerro!")),
                        _ => {  
                            eleccion.estado = ESTADO_ELECCION::CERRADA;
                            Ok(String::from("Eleccion CERRADA exitosamente. Id de la elección: ") + &eleccion.id.to_string())
                        }
                    }
                },
                None => Err(String::from("La eleccion enviada no existe!")),
            }
        }
        /// Utilizado por un administrador.
        /// cierra una elección colocando su estado en CERRADO (estado anterior al INICIADA).
        #[ink(message)] //FUNCIONA //MERJORAR
        pub fn abrir_eleccion(&mut self, eleccion_id: u64) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }

            let block_timestamp = self.env().block_timestamp();
            let eleccion_option = self.obtener_eleccion_por_id(eleccion_id);
            match eleccion_option {
                Some(eleccion) => {
                    if eleccion.fecha_inicio < block_timestamp {
                        return Err(String::from("La eleccion se encuentra en la fecha de votacion!"));
                    }
                    match eleccion.estado {
                        ESTADO_ELECCION::ABIERTA => Err(String::from("La eleccion ya se encuentra en el estado correspondiente!")),
                        ESTADO_ELECCION::INICIADA => Err(String::from("La eleccion se encuentra en el estado de votacion!")),
                        ESTADO_ELECCION::FINALIZADA => Err(String::from("La eleccion ya cerro!")),
                        _ => {  
                            eleccion.estado = ESTADO_ELECCION::ABIERTA;
                            Ok(String::from("Eleccion ABIERTA exitosamente. Id de la elección: ") + &eleccion.id.to_string())
                        }
                    }
                },
                None => Err(String::from("La eleccion enviada no existe!")),
            }
        }
        /// Utilizado por un administrador.
        /// cierra una elección colocando su estado en CERRADO (estado anterior al INICIADA).
        #[ink(message)]
        pub fn iniciar_eleccion(&mut self, eleccion_id: u64) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }

            let block_timestamp = self.env().block_timestamp();
            let eleccion_option = self.obtener_eleccion_por_id(eleccion_id);
            match eleccion_option {
                Some(eleccion) => {
                    if eleccion.fecha_inicio > block_timestamp {
                        return Err(String::from("No es la fecha propuesta, no puedes iniciarla."));
                    }
                    if eleccion.fecha_final < block_timestamp {
                        return Err(String::from("Ya finalizo, no puedes iniciarla."));
                    }
                    match eleccion.estado {
                        ESTADO_ELECCION::INICIADA => Err(String::from("La eleccion ya se encuentra en el estado correspondiente!")),
                        ESTADO_ELECCION::FINALIZADA => Err(String::from("La eleccion ya cerro!")),
                        _ => {  
                            eleccion.estado = ESTADO_ELECCION::INICIADA;
                            Ok(String::from("Eleccion INICIADA exitosamente. Id de la elección: ") + &eleccion.id.to_string())
                        }
                    }
                },
                None => Err(String::from("La eleccion enviada no existe!")),
            }
        }
        /// Utilizado por un administrador.
        /// cierra una elección colocando su estado en CERRADO (estado anterior al INICIADA).
        #[ink(message)]
        pub fn finalizar_eleccion(&mut self, eleccion_id: u64) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }

            let block_timestamp = self.env().block_timestamp();
            let eleccion_option = self.obtener_eleccion_por_id(eleccion_id);
            match eleccion_option {
                Some(eleccion) => {
                    if eleccion.fecha_final > block_timestamp {
                        return Err(String::from("No es la fecha propuesta, no puedes finalizarla."));
                    }
                    match eleccion.estado {
                        ESTADO_ELECCION::FINALIZADA => Err(String::from("La eleccion ya se encuantra en el estado correspondiente!")),
                        _ => {  
                            eleccion.estado = ESTADO_ELECCION::FINALIZADA;
                            Ok(String::from("Eleccion FINALIZADA exitosamente. Id de la elección: ") + &eleccion.id.to_string())
                        }
                    }
                },
                None => Err(String::from("La eleccion enviada no existe!")),
            }
        }

        #[ink(message)] //FUNCIONA
        pub fn obtener_ids_elecciones(&self) -> Vec<u64>
        {
            self.elecciones.iter().map(|eleccion| eleccion.id).collect()
        }

        #[ink(message)] //FUNCIONA
        pub fn obtener_datos_eleccion_por_id(&mut self, eleccion_id: u64) -> Result<String, String>
        {
            // if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            let eleccion_option = self.obtener_eleccion_por_id(eleccion_id);
            match eleccion_option {
                Some(eleccion) => {
                    let mut str = String::from("ID: ") + &eleccion.id.to_string();
                    match eleccion.estado {
                        ESTADO_ELECCION::CERRADA => str.push_str("\nEstado: CERRADA"),
                        ESTADO_ELECCION::ABIERTA => str.push_str("\nEstado: ABIERTA"),
                        ESTADO_ELECCION::INICIADA => str.push_str("\nEstado: INICIADA"),
                        ESTADO_ELECCION::FINALIZADA => str.push_str("\nEstado: FINALIZADA"),
                    }
                    str.push_str((String::from("\nfecha_inicio: ") + &eleccion.fecha_inicio.to_string()).as_str());
                    str.push_str((String::from("\nfecha_final: ") + &eleccion.fecha_final.to_string()).as_str());
                    Ok(str)
                    // Ok(String::from("Id de la elección: ") + &eleccion.id.to_string())
                },
                None => Err(String::from("La eleccion enviada no existe!")),
            }
        }
        
        #[ink(message)] //FUNCIONA
        pub fn obtener_candidatos_eleccion_por_id(&mut self, eleccion_id: u64) -> Result<String, String>
        {
            // if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            let eleccion_option = self.obtener_eleccion_por_id(eleccion_id);
            match eleccion_option {
                Some(eleccion) => {
                    if eleccion.votacion_abierta() { return Err(String::from("La eleccion no finalizo aun!")) };
                    let mut str = String::from("ID: ") + &eleccion.id.to_string();
                    eleccion.candidatos.iter().for_each(|candidato| {
                        str.push_str((String::from("\n\n")).as_str());
                        str.push_str((String::from("\nId candidato: ") + &candidato.id.to_string()).as_str());
                        str.push_str((String::from("\nId usuario: ") + hex::encode(&candidato.usuario_id).as_str() ).as_str());
                        str.push_str((String::from("\nVotos recibidos: ") + &candidato.votos_totales.to_string()).as_str());
                    });
                    Ok(str)
                    // Ok(String::from("Id de la elección: ") + &eleccion.id.to_string())
                },
                None => Err(String::from("La eleccion enviada no existe!")),
            }
        }
        #[ink(message)] //FUNCIONA
        pub fn obtener_votantes_eleccion_por_id(&mut self, eleccion_id: u64) -> Result<String, String>
        {
            // if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            let eleccion_option = self.obtener_eleccion_por_id(eleccion_id);
            match eleccion_option {
                Some(eleccion) => {
                    if eleccion.votacion_abierta() { return Err(String::from("La eleccion no finalizo aun!")) };
                    let mut str = String::from("ID: ") + &eleccion.id.to_string();
                    eleccion.votantes.iter().for_each(|votante| {
                        str.push_str((String::from("\n\n")).as_str());
                        str.push_str((String::from("\nId usuario: ") + hex::encode(&votante.usuario_id).as_str() ).as_str());
                        str.push_str((String::from("\nVotos emitido: ") + &votante.voto_emitido.to_string() ).as_str());
                    });
                    Ok(str)
                    // Ok(String::from("Id de la elección: ") + &eleccion.id.to_string())
                },
                None => Err(String::from("La eleccion enviada no existe!")),
            }
        }

    // ====================================================================
    // ====================================================================
    // ====================================================================

    // ====================================================================
    // ===================Eleccion: usuarios===============================
    // ====================================================================
        /// Utilizado por un Administrador.
        /// Obtiene la información del próximo usuario a registrarse.
        #[ink(message)] //FUNCIONA
        pub fn obtener_siguiente_usuario_pendiente_en_una_eleccion(&mut self, eleccion_id:u64) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            let eleccion_elegida = match self.obtener_eleccion_por_id(eleccion_id) {
                Some(eleccion) => eleccion,
                None => return Err(String::from("Eleccion no encontrada")),
            };
            let sig_usuario = eleccion_elegida.usuarios_pendientes.first();
            match sig_usuario {
                Some(usuario_eleccion) => {
                    let mut datos_usuario = String::from("Usuario: ");
                    datos_usuario.push_str( hex::encode(usuario_eleccion.0).as_str() );
                    match usuario_eleccion.1 {
                        TIPO_DE_USUARIO::VOTANTE => datos_usuario.push_str("\nEl usuario quiere ser un VOTANTE"),
                        TIPO_DE_USUARIO::CANDIDATO => datos_usuario.push_str("\nEl usuario quiere ser un CANDIDATO")
                    };
                    Ok(datos_usuario)
                },
                None => Err(String::from("No hay usuarios pendientes.")),
            }
        }
        /// Utilizado por un Administrador.
        /// Se procesará el próximo usuario pendiente en una eleccion particular.
        /// y se lo coloca en el vector de candidato o votante en esa eleccion segun que quiera ser.
        #[ink(message)] //FUNCIONA
        pub fn procesar_usuarios_en_una_eleccion(&mut self, eleccion_id:u64,aceptar_usuario:bool) -> Result<String, String>
        {
                if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }

               let eleccion_elegida = match self.obtener_eleccion_por_id(eleccion_id) {
                Some(eleccion) => eleccion,
                None => return Err(String::from("Eleccion no encontrada")),
            };
            eleccion_elegida.procesar_siguiente_usuario_pendiente(aceptar_usuario)
        }



        // inscribir_usuario_en_eleccion (deben ser usuarios del sistema  y no estar ya en la eleccion)
        #[ink(message)] //FUNCIONA
        pub fn inscribir_usuario_en_eleccion(&mut self, eleccion_id:u64, tipo:TIPO_DE_USUARIO) -> Result<String, String>
        {
            // es usuario valido en el sistema (no esta pendiente de aprobacion y no esta rechazado)
            let id = self.env().caller();
            if !self.es_usuario_registrado(id) { return Err(ERRORES::USUARIO_NO_REGISTRADO.to_string()); }

            // el estado de la eleccion es ABIERTA
            let block_timestamp = self.env().block_timestamp();
            let result = self.validar_estado_eleccion_para_inscripciones(eleccion_id, block_timestamp);
            let eleccion = match result {
                Ok(eleccion) => eleccion,
                Err(mensaje) => return Err(mensaje)
            };

            //Validar que un usuario que ya ha sido rechazado en la misma eleccion no intente volver a ponerse como pendiente 
            if eleccion.es_usuario_rechazado(id) {  return Err(String::from("Tu solicitud de registro ya fue rechazada.")); }
            if eleccion.es_usuario_pendiente(id) { return Err(String::from("Ya estás en la cola de usuarios pendientes.")); }
            if eleccion.es_votante(id) { return Err(String::from("Ya estás registrado como votante."));}
            if eleccion.es_candidato(id) {return Err("Ya has sido aceptado como candidato".to_string());}
            
            eleccion.usuarios_pendientes.push((id,tipo));
            return Ok("Ingresó a la elección correctamente Pendiente de aprobacion del Administrador".to_string());
        }


        /// Utilizado por los usuarios registrados en el sistema y que están en la elección ingresada.
        /// Se utiliza para poder obtener información de algún candidato en específico.
        /// Las IDs de los candidatos van de 1 a N.
        #[ink(message)] //FUNCIONA
        pub fn obtener_informacion_candidato(&mut self, eleccion_id: u64, candidato_id: u64) -> Result<String, String> {
            let eleccion_elegida = self.obtener_eleccion_por_id(eleccion_id)
                .ok_or_else(|| String::from("No existe una elección con ese id."))?;
            
            let candidato_elegido = eleccion_elegida.obtener_informacion_candidato(candidato_id)
                .ok_or_else(|| String::from("No existe un candidato con ese id."))?;
            
            let usuario_id = candidato_elegido.usuario_id;
            let usuario = self.obtener_usuario_por_id(usuario_id)
                .ok_or_else(|| String::from("No existe un usuario/candidato con ese id."))?;
        
            let informacion = format!("Nombre: {}\nApellido: {}", usuario.nombre, usuario.apellido);
            
            Ok(informacion)
        }

        /// Utilizado por los usuarios registrados en el sistema y que están en la elección como votantes.
        /// Si el usuario ya emitió su voto, no puede volver a votar en la misma elección.
        /// Si el usuario no es votante, no puede votar.
        /// Si el periodo de la votación no comenzó o terminó, no puede votar.
        #[ink(message)] 
        pub fn votar_a_candidato(&mut self, eleccion_id:u64, candidato_id:u64) -> Result<String, String>
        {
            // es usuario valido en el sistema (no esta pendiente de aprobacion y no esta rechazado)
            let id = self.env().caller();
            if !self.es_usuario_registrado(id) { return Err(ERRORES::USUARIO_NO_REGISTRADO.to_string()); }

            // el estado de la eleccion es ABIERTA
            let block_timestamp = self.env().block_timestamp();
            let result = self.validar_estado_eleccion_para_votaciones(eleccion_id, block_timestamp);
            let eleccion = match result {
                Ok(eleccion) => eleccion,
                Err(mensaje) => return Err(mensaje)
            };

            //Validar que un usuario que ya ha sido rechazado en la misma eleccion no intente volver a ponerse como pendiente 
            if eleccion.es_usuario_rechazado(id) {  return Err(String::from("Tu solicitud de registro fue rechazada.")); }
            if eleccion.es_usuario_pendiente(id) { return Err(String::from("Estás en la cola de usuarios pendientes.")); }
            if !eleccion.es_votante(id) { return Err(String::from("No estás registrado como votante."));}
            if eleccion.es_candidato(id) {return Err("Has sido aceptado como candidato".to_string());}

            return eleccion.votar_candidato(id, candidato_id);
        }

    // ====================================================================
    // ====================================================================
    // ====================================================================



        /// Utilizado por el administrador.
        /// Permite al administrador transferir el rol de administrador a otra persona.
        #[ink(message)] //FUNCIONA
        pub fn transferir_administrador(&mut self, id:AccountId) -> Result<String, String>
        {
            if !self.es_administrador() { return Err(ERRORES::NO_ES_ADMINISTRADOR.to_string()); }
            self.administrador = id;
            return Ok(String::from("Se transfirió el rol de administrador correctamente."));
        }

    }
   
        


    /*#[cfg(test)]
    mod tests {
        use super::*;
    }*/
}

