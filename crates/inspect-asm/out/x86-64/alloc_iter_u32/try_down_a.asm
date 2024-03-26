inspect_asm::alloc_iter_u32::try_down_a:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	mov qword ptr [rsp + 32], rsi
	test rdx, rdx
	je .LBB_1
	mov r8, rdx
	cmp rdx, 5
	mov r15d, 4
	cmovae r15, rdx
	mov rax, rdx
	shr rax, 61
	je .LBB_4
.LBB_3:
	xor eax, eax
	jmp .LBB_23
.LBB_1:
	mov eax, 4
	xor edx, edx
	jmp .LBB_23
.LBB_4:
	lea rdx, [4*r15]
	mov rax, qword ptr [rdi]
	mov rcx, qword ptr [rax]
	xor r13d, r13d
	sub rcx, rdx
	cmovae r13, rcx
	and r13, -4
	cmp r13, qword ptr [rax + 8]
	jb .LBB_6
	mov qword ptr [rax], r13
.LBB_7:
	mov qword ptr [rsp], rdi
	shl r8, 2
	xor ebx, ebx
	xor edx, edx
	mov qword ptr [rsp + 16], r8
	jmp .LBB_8
.LBB_51:
	mov r15, r14
	mov r13, rbp
.LBB_71:
	mov dword ptr [r13 + 4*r12], esi
	add rbx, 4
	cmp r8, rbx
	je .LBB_11
.LBB_8:
	mov r12, rdx
	mov rax, qword ptr [rsp + 32]
	mov esi, dword ptr [rax + rbx]
	cmp r15, rdx
	jne .LBB_9
	lea rax, [r15 + 1]
	lea rcx, [r15 + r15]
	cmp rcx, rax
	cmova rax, rcx
	mov r14, rax
	cmp rax, 5
	jae .LBB_26
	mov r14d, 4
.LBB_26:
	lea rdx, [4*r14]
	movabs rcx, 2305843009213693951
	test r15, r15
	je .LBB_34
	cmp rax, rcx
	ja .LBB_79
	lea rcx, [4*r15]
	mov rax, qword ptr [rsp]
	mov rax, qword ptr [rax]
	mov rbp, qword ptr [rax]
	cmp rbp, r13
	je .LBB_36
	sub rbp, rdx
	jae .LBB_31
	xor ebp, ebp
.LBB_31:
	and rbp, -4
	cmp rbp, qword ptr [rax + 8]
	jb .LBB_78
	mov qword ptr [rax], rbp
.LBB_33:
	mov rdi, rbp
	mov r15d, esi
	mov rsi, r13
	mov rdx, rcx
	call qword ptr [rip + memcpy@GOTPCREL]
	mov esi, r15d
	mov r8, qword ptr [rsp + 16]
	jmp .LBB_50
.LBB_9:
	mov r14, r15
	mov rbp, r13
	jmp .LBB_50
.LBB_34:
	cmp rax, rcx
	ja .LBB_35
	mov rax, qword ptr [rsp]
	mov rax, qword ptr [rax]
	mov rbp, qword ptr [rax]
	sub rbp, rdx
	jae .LBB_47
	xor ebp, ebp
.LBB_47:
	and rbp, -4
	cmp rbp, qword ptr [rax + 8]
	jb .LBB_49
	mov qword ptr [rax], rbp
	jmp .LBB_50
.LBB_36:
	mov dword ptr [rsp + 8], esi
	mov rsi, rdx
	sub rsi, rcx
	mov rbp, r13
	sub rbp, rsi
	jae .LBB_38
	xor ebp, ebp
.LBB_38:
	and rbp, -4
	cmp rbp, qword ptr [rax + 8]
	jb .LBB_39
	add rdx, rbp
	mov rdi, rbp
	mov rsi, r13
	cmp rdx, r13
	jae .LBB_42
	mov rdx, rcx
	call qword ptr [rip + memcpy@GOTPCREL]
	jmp .LBB_44
.LBB_42:
	mov rdx, rcx
	call qword ptr [rip + memmove@GOTPCREL]
.LBB_44:
	mov rax, qword ptr [rsp]
	mov rax, qword ptr [rax]
	mov qword ptr [rax], rbp
	mov r8, qword ptr [rsp + 16]
	mov esi, dword ptr [rsp + 8]
.LBB_50:
	lea rdx, [r12 + 1]
	cmp r12, r14
	jne .LBB_51
	lea rax, [r12 + r12]
	cmp rax, rdx
	cmovbe rax, rdx
	cmp rax, 5
	mov r15d, 4
	cmovae r15, rax
	lea r9, [4*r15]
	movabs rcx, 2305843009213693951
	test r12, r12
	je .LBB_76
	cmp rax, rcx
	ja .LBB_77
	lea r8, [4*r12]
	mov rax, qword ptr [rsp]
	mov rax, qword ptr [rax]
	mov rcx, qword ptr [rax]
	cmp rcx, rbp
	mov qword ptr [rsp + 8], rdx
	je .LBB_59
	mov r14d, esi
	sub rcx, r9
	mov r13d, 0
	cmovae r13, rcx
	and r13, -4
	cmp r13, qword ptr [rax + 8]
	jb .LBB_74
	mov qword ptr [rax], r13
.LBB_57:
	mov rdi, r13
	mov rsi, rbp
	mov rdx, r8
	call qword ptr [rip + memcpy@GOTPCREL]
	jmp .LBB_68
.LBB_76:
	cmp rax, rcx
	ja .LBB_77
	mov rax, qword ptr [rsp]
	mov rax, qword ptr [rax]
	mov rcx, qword ptr [rax]
	sub rcx, r9
	mov r13d, 0
	cmovae r13, rcx
	and r13, -4
	cmp r13, qword ptr [rax + 8]
	jb .LBB_72
	mov qword ptr [rax], r13
	mov edx, 1
	jmp .LBB_71
.LBB_59:
	mov rcx, r9
	sub rcx, r8
	mov r13, rbp
	sub r13, rcx
	jae .LBB_61
	xor r13d, r13d
.LBB_61:
	and r13, -4
	cmp r13, qword ptr [rax + 8]
	jb .LBB_62
	mov r14d, esi
	add r9, r13
	mov rdi, r13
	mov rsi, rbp
	mov rdx, r8
	cmp r9, rbp
	jae .LBB_65
	call qword ptr [rip + memcpy@GOTPCREL]
	jmp .LBB_67
.LBB_65:
	call qword ptr [rip + memmove@GOTPCREL]
.LBB_67:
	mov rax, qword ptr [rsp]
	mov rax, qword ptr [rax]
	mov qword ptr [rax], r13
.LBB_68:
	mov r8, qword ptr [rsp + 16]
	mov esi, r14d
	mov rdx, qword ptr [rsp + 8]
	jmp .LBB_71
.LBB_74:
	mov esi, 4
	mov rdi, qword ptr [rsp]
	mov rdx, r9
	mov qword ptr [rsp + 24], r9
	mov r13, r8
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov r8, r13
	mov r13, rax
	test rax, rax
	jne .LBB_57
	jmp .LBB_75
.LBB_78:
	mov dword ptr [rsp + 8], esi
	mov esi, 4
	mov rdi, qword ptr [rsp]
	mov rbp, rcx
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov esi, dword ptr [rsp + 8]
	mov rcx, rbp
	mov rbp, rax
	test rax, rax
	jne .LBB_33
	jmp .LBB_79
.LBB_72:
	mov r14d, esi
	mov esi, 4
	mov rdi, qword ptr [rsp]
	mov rdx, r9
	mov qword ptr [rsp + 24], r9
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	test rax, rax
	je .LBB_75
	mov r13, rax
	mov edx, 1
	mov r8, qword ptr [rsp + 16]
	mov esi, r14d
	jmp .LBB_71
.LBB_49:
	mov ebp, esi
	mov esi, 4
	mov rdi, qword ptr [rsp]
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov esi, ebp
	mov r8, qword ptr [rsp + 16]
	mov rbp, rax
	test rax, rax
	jne .LBB_50
	jmp .LBB_35
.LBB_62:
	mov r14d, esi
	mov esi, 4
	mov rdi, qword ptr [rsp]
	mov rdx, r9
	mov qword ptr [rsp + 24], r9
	mov r13, r8
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	test rax, rax
	je .LBB_75
	mov rdx, r13
	mov r13, rax
	mov rdi, rax
	mov rsi, rbp
	call qword ptr [rip + memcpy@GOTPCREL]
	jmp .LBB_68
.LBB_39:
	mov esi, 4
	mov rdi, qword ptr [rsp]
	mov rbp, rcx
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	test rax, rax
	je .LBB_79
	mov rdx, rbp
	mov rbp, rax
	mov rdi, rax
	mov rsi, r13
	call qword ptr [rip + memcpy@GOTPCREL]
	mov esi, dword ptr [rsp + 8]
	mov r8, qword ptr [rsp + 16]
	jmp .LBB_50
.LBB_11:
	cmp r15, rdx
	jbe .LBB_12
	lea rax, [4*r15]
	mov rsi, rdx
	lea rdx, [4*rdx]
	test r13b, 3
	jne .LBB_14
	mov rbx, qword ptr [rsp]
	mov rcx, qword ptr [rbx]
	cmp qword ptr [rcx], r13
	je .LBB_19
	mov rax, r13
	mov rdx, rsi
	jmp .LBB_23
.LBB_12:
	mov rax, r13
	jmp .LBB_23
.LBB_79:
	mov rax, qword ptr [rsp]
	mov rax, qword ptr [rax]
	cmp qword ptr [rax], r13
	jne .LBB_3
	lea rcx, [4*r15 + 3]
	add rcx, r13
	and rcx, -4
	mov qword ptr [rax], rcx
	xor eax, eax
	jmp .LBB_23
.LBB_35:
	xor eax, eax
	jmp .LBB_23
.LBB_19:
	mov r15, rsi
	add rax, r13
	xor edi, edi
	sub rax, rdx
	cmovae rdi, rax
	lea rax, [rdx + r13]
	mov r14, rdi
	mov rsi, r13
	cmp rax, rdi
	jbe .LBB_20
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB_22
.LBB_6:
	mov esi, 4
	mov rbx, rdi
	mov r14, r8
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov r8, r14
	mov rdi, rbx
	mov r13, rax
	test rax, rax
	jne .LBB_7
	jmp .LBB_3
.LBB_20:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB_22:
	mov rax, qword ptr [rbx]
	mov qword ptr [rax], r14
	mov rax, r14
	mov rdx, r15
.LBB_23:
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB_14:
	mov ecx, 4
	mov r14, rsi
	mov rdi, qword ptr [rsp]
	mov rsi, r13
	mov r8, rdx
	mov rdx, rax
	mov rbx, r8
	call bump_scope::allocator::shrink::shrink_unfit
	mov rdx, r14
	test rax, rax
	jne .LBB_23
	mov edi, 4
	mov rsi, rbx
	call qword ptr [rip + alloc::alloc::handle_alloc_error@GOTPCREL]
	jmp .LBB_16
.LBB_77:
	call qword ptr [rip + allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL]
	jmp .LBB_16
.LBB_75:
	mov edi, 4
	mov rsi, qword ptr [rsp + 24]
	call qword ptr [rip + alloc::alloc::handle_alloc_error@GOTPCREL]
.LBB_16:
	ud2
	mov rbp, r13
	mov r12, r15
	jmp .LBB_82
	test r12, r12
	je .LBB_84
.LBB_82:
	mov rcx, qword ptr [rsp]
	mov rcx, qword ptr [rcx]
	cmp qword ptr [rcx], rbp
	jne .LBB_84
	lea rdx, [4*r12 + 3]
	add rdx, rbp
	and rdx, -4
	mov qword ptr [rcx], rdx
.LBB_84:
	mov rdi, rax
	call _Unwind_Resume@PLT