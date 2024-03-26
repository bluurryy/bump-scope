inspect_asm::alloc_iter_u32::up:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	test rdx, rdx
	je .LBB_1
	mov r9, rdx
	cmp rdx, 5
	mov ebp, 4
	mov r8d, 4
	cmovae r8, rdx
	mov rax, rdx
	shr rax, 61
	mov qword ptr [rsp], rdi
	jne .LBB_10
	lea rbx, [4*r8]
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	dec rax
	and rax, -4
	lea r10, [rbx + 4]
	add r10, rax
	mov rdx, -1
	cmovae rdx, r10
	cmp rdx, qword ptr [rcx + 8]
	mov qword ptr [rsp + 8], rsi
	ja .LBB_6
	add rax, 4
	mov qword ptr [rcx], rdx
.LBB_7:
	movabs rdi, 2305843009213693951
	shl r9, 2
	neg r9
	xor r13d, r13d
	mov rbp, rax
	xor r12d, r12d
	mov qword ptr [rsp + 32], r9
	jmp .LBB_8
.LBB_9:
	mov r8, r14
	mov rbp, rbx
.LBB_33:
	mov dword ptr [rbp + 4*r12], r15d
	inc r12
	add r13, -4
	cmp r9, r13
	je .LBB_14
.LBB_8:
	mov r14, r8
	mov rbx, rbp
	mov r15d, dword ptr [rsi + 4*r12]
	cmp r12, r8
	jne .LBB_9
	lea rax, [r14 + 1]
	lea rcx, [r14 + r14]
	cmp rcx, rax
	cmova rax, rcx
	mov r8, rax
	cmp rax, 5
	jae .LBB_23
	mov r8d, 4
.LBB_23:
	lea r10, [4*r8]
	test r14, r14
	je .LBB_43
	cmp rax, rdi
	ja .LBB_44
	lea r11, [4*r14]
	mov rax, qword ptr [rsp]
	mov rax, qword ptr [rax]
	mov rbp, qword ptr [rax]
	mov rcx, qword ptr [rax + 8]
	test bl, 3
	jne .LBB_27
	lea rdx, [rbx + r11]
	cmp rdx, rbp
	jne .LBB_27
	sub rcx, rbx
	cmp rcx, r10
	mov qword ptr [rsp + 24], r10
	jb .LBB_36
	lea rcx, [r10 + rbx]
	mov qword ptr [rax], rcx
	mov rbp, rbx
	test rbp, rbp
	jne .LBB_33
	jmp .LBB_42
.LBB_43:
	cmp rax, rdi
	ja .LBB_44
	mov rax, qword ptr [rsp]
	mov rax, qword ptr [rax]
	mov rbp, qword ptr [rax]
	dec rbp
	and rbp, -4
	mov rcx, rbp
	add rcx, 20
	mov rdx, -1
	cmovb rcx, rdx
	cmp rcx, qword ptr [rax + 8]
	ja .LBB_40
	add rbp, 4
	mov qword ptr [rax], rcx
	jmp .LBB_33
.LBB_27:
	dec rbp
	and rbp, -4
	lea rsi, [r10 + 4]
	mov rdx, -1
	add rsi, rbp
	jb .LBB_29
	mov rdx, rsi
.LBB_29:
	cmp rdx, rcx
	ja .LBB_31
	add rbp, 4
	mov qword ptr [rax], rdx
.LBB_32:
	mov rdi, rbp
	mov rsi, rbx
	mov rdx, r11
	mov rbx, r8
	call qword ptr [rip + memcpy@GOTPCREL]
	movabs rdi, 2305843009213693951
	mov rsi, qword ptr [rsp + 8]
	mov r9, qword ptr [rsp + 32]
	mov r8, rbx
	jmp .LBB_33
.LBB_40:
	mov esi, 4
	mov rdi, qword ptr [rsp]
	mov qword ptr [rsp + 24], r10
	mov rdx, r10
	mov rbp, r8
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	movabs rdi, 2305843009213693951
	mov rsi, qword ptr [rsp + 8]
	mov r9, qword ptr [rsp + 32]
	mov r8, rbp
	mov rbp, rax
	test rbp, rbp
	jne .LBB_33
	jmp .LBB_42
.LBB_31:
	mov esi, 4
	mov rdi, qword ptr [rsp]
	mov qword ptr [rsp + 24], r10
	mov rdx, r10
	mov rbp, r8
	mov qword ptr [rsp + 16], r11
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov r11, qword ptr [rsp + 16]
	mov r8, rbp
	mov rbp, rax
	test rax, rax
	jne .LBB_32
	jmp .LBB_42
.LBB_36:
	mov esi, 4
	mov rdi, qword ptr [rsp]
	mov rdx, r10
	mov qword ptr [rsp + 16], r8
	mov rbp, r11
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	test rax, rax
	je .LBB_42
	mov rdx, rbp
	mov rbp, rax
	mov rdi, rax
	mov rsi, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
	movabs rdi, 2305843009213693951
	mov rsi, qword ptr [rsp + 8]
	mov r9, qword ptr [rsp + 32]
	mov r8, qword ptr [rsp + 16]
	jmp .LBB_33
.LBB_14:
	cmp r8, r12
	jbe .LBB_20
	lea rdx, [4*r8]
	test bpl, 3
	jne .LBB_16
	add rdx, rbp
	mov rax, qword ptr [rsp]
	mov rax, qword ptr [rax]
	cmp rdx, qword ptr [rax]
	jne .LBB_20
	mov rcx, rbp
	sub rcx, r13
	mov qword ptr [rax], rcx
.LBB_20:
	mov rax, rbp
	jmp .LBB_2
.LBB_1:
	mov eax, 4
	xor r12d, r12d
.LBB_2:
	mov rdx, r12
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB_6:
	mov esi, 4
	mov rdx, rbx
	mov r14, r8
	mov r15, r9
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov rsi, qword ptr [rsp + 8]
	mov r9, r15
	mov r8, r14
	test rax, rax
	jne .LBB_7
	xor r14d, r14d
	mov edi, 4
	mov rsi, rbx
	call qword ptr [rip + alloc::alloc::handle_alloc_error@GOTPCREL]
	jmp .LBB_11
.LBB_16:
	mov r14, r8
	neg r13
	mov ecx, 4
	mov rdi, qword ptr [rsp]
	mov rsi, rbp
	mov r8, r13
	call bump_scope::allocator::shrink::shrink_unfit
	test rax, rax
	jne .LBB_2
	mov edi, 4
	mov rsi, r13
	call qword ptr [rip + alloc::alloc::handle_alloc_error@GOTPCREL]
	jmp .LBB_11
.LBB_44:
	call qword ptr [rip + allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL]
	jmp .LBB_11
.LBB_42:
	mov edi, 4
	mov rsi, qword ptr [rsp + 24]
	call qword ptr [rip + alloc::alloc::handle_alloc_error@GOTPCREL]
	jmp .LBB_11
.LBB_10:
	xor r14d, r14d
	call qword ptr [rip + allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL]
.LBB_11:
	ud2
	mov rbx, rbp
	jmp .LBB_46
.LBB_46:
	test r14, r14
	je .LBB_49
	lea rdx, [rbx + 4*r14]
	mov rcx, qword ptr [rsp]
	mov rcx, qword ptr [rcx]
	cmp rdx, qword ptr [rcx]
	jne .LBB_49
	mov qword ptr [rcx], rbx
.LBB_49:
	mov rdi, rax
	call _Unwind_Resume@PLT