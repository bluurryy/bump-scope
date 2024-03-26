inspect_asm::alloc_iter_u32::down:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 56
	test rdx, rdx
	je .LBB_38
	mov r8, rdx
	cmp rdx, 5
	mov r13d, 4
	mov r9d, 4
	cmovae r9, rdx
	mov rax, rdx
	shr rax, 61
	mov qword ptr [rsp + 8], rdi
	jne .LBB_50
	lea rbx, [4*r9]
	mov rcx, qword ptr [rdi]
	mov rdx, qword ptr [rcx]
	xor eax, eax
	sub rdx, rbx
	cmovae rax, rdx
	and rax, -4
	cmp rax, qword ptr [rcx + 8]
	mov qword ptr [rsp + 24], rsi
	jb .LBB_45
	mov qword ptr [rcx], rax
.LBB_4:
	shl r8, 2
	neg r8
	xor r12d, r12d
	mov r13, rax
	xor r14d, r14d
	mov qword ptr [rsp + 40], r8
	jmp .LBB_7
.LBB_5:
	mov r9, r15
	mov r13, rbp
.LBB_6:
	mov dword ptr [r13 + 4*r14], edx
	inc r14
	add r12, -4
	cmp r8, r12
	je .LBB_34
.LBB_7:
	mov r15, r9
	mov rbp, r13
	mov edx, dword ptr [rsi + 4*r14]
	cmp r14, r9
	jne .LBB_5
	lea rax, [r15 + 1]
	lea rcx, [r15 + r15]
	cmp rcx, rax
	cmova rax, rcx
	mov r9, rax
	cmp rax, 5
	jae .LBB_10
	mov r9d, 4
.LBB_10:
	lea rbx, [4*r9]
	movabs rcx, 2305843009213693951
	test r15, r15
	je .LBB_18
	cmp rax, rcx
	ja .LBB_49
	lea r8, [4*r15]
	mov rax, qword ptr [rsp + 8]
	mov rax, qword ptr [rax]
	mov r13, qword ptr [rax]
	cmp r13, rbp
	je .LBB_23
	sub r13, rbx
	jae .LBB_15
	xor r13d, r13d
.LBB_15:
	and r13, -4
	cmp r13, qword ptr [rax + 8]
	jb .LBB_30
	mov qword ptr [rax], r13
.LBB_17:
	mov rdi, r13
	mov rbx, r9
	mov rsi, rbp
	mov ebp, edx
	mov rdx, r8
	call qword ptr [rip + memcpy@GOTPCREL]
	mov edx, ebp
	mov rsi, qword ptr [rsp + 24]
	mov r8, qword ptr [rsp + 40]
	mov r9, rbx
	jmp .LBB_6
.LBB_18:
	cmp rax, rcx
	ja .LBB_49
	mov rax, qword ptr [rsp + 8]
	mov rax, qword ptr [rax]
	mov r13, qword ptr [rax]
	sub r13, rbx
	jae .LBB_21
	xor r13d, r13d
.LBB_21:
	and r13, -4
	cmp r13, qword ptr [rax + 8]
	jb .LBB_31
	mov qword ptr [rax], r13
	jmp .LBB_6
.LBB_23:
	mov rcx, rbx
	sub rcx, r8
	mov r13, rbp
	sub r13, rcx
	jae .LBB_25
	xor r13d, r13d
.LBB_25:
	and r13, -4
	cmp r13, qword ptr [rax + 8]
	mov dword ptr [rsp + 36], edx
	jb .LBB_32
	mov r15, r9
	add rbx, r13
	mov rdi, r13
	mov rsi, rbp
	mov rdx, r8
	cmp rbx, rbp
	jae .LBB_28
	call qword ptr [rip + memcpy@GOTPCREL]
	jmp .LBB_29
.LBB_28:
	call qword ptr [rip + memmove@GOTPCREL]
.LBB_29:
	mov rax, qword ptr [rsp + 8]
	mov rax, qword ptr [rax]
	mov qword ptr [rax], r13
	mov r9, r15
	mov r8, qword ptr [rsp + 40]
	mov rsi, qword ptr [rsp + 24]
	mov edx, dword ptr [rsp + 36]
	jmp .LBB_6
.LBB_30:
	mov qword ptr [rsp + 16], r9
	mov esi, 4
	mov rdi, qword ptr [rsp + 8]
	mov r13d, edx
	mov rdx, rbx
	mov qword ptr [rsp + 48], r8
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov r8, qword ptr [rsp + 48]
	mov edx, r13d
	mov r9, qword ptr [rsp + 16]
	mov r13, rax
	test rax, rax
	jne .LBB_17
	jmp .LBB_51
.LBB_31:
	mov qword ptr [rsp + 16], r9
	mov esi, 4
	mov rdi, qword ptr [rsp + 8]
	mov r13d, edx
	mov rdx, rbx
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov edx, r13d
	mov rsi, qword ptr [rsp + 24]
	mov r8, qword ptr [rsp + 40]
	mov r9, qword ptr [rsp + 16]
	mov r13, rax
	test rax, rax
	jne .LBB_6
	jmp .LBB_51
.LBB_32:
	mov qword ptr [rsp + 16], r9
	mov esi, 4
	mov rdi, qword ptr [rsp + 8]
	mov rdx, rbx
	mov r13, r8
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	test rax, rax
	je .LBB_51
	mov rdx, r13
	mov r13, rax
	mov rdi, rax
	mov rsi, rbp
	call qword ptr [rip + memcpy@GOTPCREL]
	mov edx, dword ptr [rsp + 36]
	mov rsi, qword ptr [rsp + 24]
	mov r8, qword ptr [rsp + 40]
	mov r9, qword ptr [rsp + 16]
	jmp .LBB_6
.LBB_34:
	cmp r9, r14
	jbe .LBB_39
	lea rax, [4*r9]
	mov rdx, r12
	neg rdx
	test r13b, 3
	jne .LBB_47
	mov r15, qword ptr [rsp + 8]
	mov rcx, qword ptr [r15]
	cmp qword ptr [rcx], r13
	je .LBB_40
.LBB_39:
	mov rax, r13
	jmp .LBB_44
.LBB_38:
	mov eax, 4
	xor r14d, r14d
	jmp .LBB_44
.LBB_40:
	add rax, r13
	xor edi, edi
	sub rax, rdx
	cmovae rdi, rax
	mov rax, r13
	sub rax, r12
	mov rbx, rdi
	mov rsi, r13
	cmp rax, rdi
	jbe .LBB_42
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB_43
.LBB_42:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB_43:
	mov rcx, qword ptr [r15]
	mov rax, rbx
	mov qword ptr [rcx], rbx
.LBB_44:
	mov rdx, r14
	add rsp, 56
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB_45:
	mov r14, r9
	mov esi, 4
	mov rdx, rbx
	mov r15, r8
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov rsi, qword ptr [rsp + 24]
	mov r8, r15
	mov r9, r14
	test rax, rax
	jne .LBB_4
	xor r15d, r15d
	mov edi, 4
	mov rsi, rbx
	call qword ptr [rip + alloc::alloc::handle_alloc_error@GOTPCREL]
	jmp .LBB_52
.LBB_47:
	mov r15, r9
	mov ecx, 4
	mov rdi, qword ptr [rsp + 8]
	mov rsi, r13
	mov r8, rdx
	mov rdx, rax
	mov rbx, r8
	call bump_scope::allocator::shrink::shrink_unfit
	test rax, rax
	jne .LBB_44
	mov edi, 4
	mov rsi, rbx
	call qword ptr [rip + alloc::alloc::handle_alloc_error@GOTPCREL]
	jmp .LBB_52
.LBB_49:
	call qword ptr [rip + allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL]
	jmp .LBB_52
.LBB_50:
	xor r15d, r15d
	call qword ptr [rip + allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL]
	jmp .LBB_52
.LBB_51:
	mov edi, 4
	mov rsi, rbx
	call qword ptr [rip + alloc::alloc::handle_alloc_error@GOTPCREL]
.LBB_52:
	ud2
	mov rbp, r13
	jmp .LBB_55
.LBB_55:
	test r15, r15
	je .LBB_58
	mov rcx, qword ptr [rsp + 8]
	mov rcx, qword ptr [rcx]
	cmp qword ptr [rcx], rbp
	jne .LBB_58
	lea rdx, [4*r15]
	add rdx, rbp
	mov qword ptr [rcx], rdx
.LBB_58:
	mov rdi, rax
	call _Unwind_Resume@PLT