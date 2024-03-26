inspect_asm::alloc_iter_u32::try_up_a:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 56
	test rdx, rdx
	je .LBB_1
	mov r8, rdx
	cmp rdx, 5
	mov r13d, 4
	cmovae r13, rdx
	mov rax, rdx
	shr rax, 61
	je .LBB_5
.LBB_4:
	xor eax, eax
	jmp .LBB_2
.LBB_1:
	mov eax, 4
	xor edi, edi
	jmp .LBB_2
.LBB_5:
	lea rdx, [4*r13]
	mov rax, qword ptr [rdi]
	mov rbp, qword ptr [rax]
	dec rbp
	and rbp, -4
	lea r9, [rdx + 4]
	add r9, rbp
	mov rcx, -1
	cmovae rcx, r9
	cmp rcx, qword ptr [rax + 8]
	mov qword ptr [rsp + 16], rsi
	ja .LBB_7
	add rbp, 4
	add rcx, 3
	and rcx, -4
	mov qword ptr [rax], rcx
.LBB_8:
	mov qword ptr [rsp + 8], rdi
	movabs r9, 2305843009213693951
	shl r8, 2
	xor r15d, r15d
	xor edi, edi
	mov qword ptr [rsp + 32], r8
	jmp .LBB_9
.LBB_43:
	mov r13, r14
	mov rbp, r12
.LBB_59:
	mov dword ptr [rbp + 4*rbx], r10d
	add r15, 4
	cmp r8, r15
	je .LBB_12
.LBB_9:
	mov rbx, rdi
	mov r10d, dword ptr [rsi + r15]
	cmp r13, rdi
	jne .LBB_10
	lea rax, [r13 + 1]
	lea rcx, [2*r13]
	cmp rcx, rax
	cmova rax, rcx
	mov r14, rax
	cmp rax, 5
	jae .LBB_22
	mov r14d, 4
.LBB_22:
	lea rdx, [4*r14]
	test r13, r13
	je .LBB_32
	shl r13, 2
	cmp rax, r9
	ja .LBB_66
	mov rax, qword ptr [rsp + 8]
	mov rax, qword ptr [rax]
	mov r12, qword ptr [rax]
	mov rcx, qword ptr [rax + 8]
	test bpl, 3
	jne .LBB_26
	mov rdi, rbp
	add rdi, r13
	cmp rdi, r12
	jne .LBB_26
	sub rcx, rbp
	cmp rcx, rdx
	jb .LBB_34
	lea rcx, [rdx + rbp]
	add rcx, 3
	and rcx, -4
	mov qword ptr [rax], rcx
	test rbp, rbp
	je .LBB_66
	mov r12, rbp
	jmp .LBB_42
.LBB_10:
	mov r14, r13
	mov r12, rbp
	jmp .LBB_42
.LBB_32:
	cmp rax, r9
	ja .LBB_33
	mov rax, qword ptr [rsp + 8]
	mov rax, qword ptr [rax]
	mov r12, qword ptr [rax]
	dec r12
	and r12, -4
	mov rcx, r12
	add rcx, 20
	mov rdi, -1
	cmovb rcx, rdi
	cmp rcx, qword ptr [rax + 8]
	ja .LBB_41
	add r12, 4
	add rcx, 3
	and rcx, -4
	mov qword ptr [rax], rcx
	jmp .LBB_42
.LBB_26:
	dec r12
	and r12, -4
	lea rdi, [rdx + 4]
	mov rsi, -1
	add rdi, r12
	jb .LBB_28
	mov rsi, rdi
.LBB_28:
	cmp rsi, rcx
	ja .LBB_65
	add r12, 4
	add rsi, 3
	and rsi, -4
	mov qword ptr [rax], rsi
.LBB_30:
	mov rdi, r12
	mov rsi, rbp
	mov rdx, r13
	mov ebp, r10d
	call qword ptr [rip + memcpy@GOTPCREL]
	mov r10d, ebp
.LBB_31:
	movabs r9, 2305843009213693951
	mov rsi, qword ptr [rsp + 16]
	mov r8, qword ptr [rsp + 32]
.LBB_42:
	lea rdi, [rbx + 1]
	cmp rbx, r14
	jne .LBB_43
	lea rax, [rbx + rbx]
	cmp rax, rdi
	cmovbe rax, rdi
	cmp rax, 5
	mov r13d, 4
	cmovae r13, rax
	lea r11, [4*r13]
	test rbx, rbx
	je .LBB_63
	cmp rax, r9
	ja .LBB_64
	lea r14, [4*rbx]
	mov rax, qword ptr [rsp + 8]
	mov rax, qword ptr [rax]
	mov rbp, qword ptr [rax]
	mov rcx, qword ptr [rax + 8]
	test r12b, 3
	jne .LBB_48
	lea rdx, [r12 + r14]
	cmp rdx, rbp
	jne .LBB_48
	sub rcx, r12
	cmp rcx, r11
	mov qword ptr [rsp + 48], r11
	jb .LBB_55
	lea rcx, [r12 + r11]
	add rcx, 3
	and rcx, -4
	mov qword ptr [rax], rcx
	mov rbp, r12
	test rbp, rbp
	jne .LBB_59
	jmp .LBB_62
.LBB_63:
	cmp rax, r9
	ja .LBB_64
	mov rax, qword ptr [rsp + 8]
	mov rax, qword ptr [rax]
	mov rbp, qword ptr [rax]
	dec rbp
	and rbp, -4
	mov rcx, rbp
	add rcx, 20
	mov rdx, -1
	cmovb rcx, rdx
	cmp rcx, qword ptr [rax + 8]
	ja .LBB_60
	add rbp, 4
	add rcx, 3
	and rcx, -4
	mov qword ptr [rax], rcx
	mov edi, 1
	jmp .LBB_59
.LBB_48:
	dec rbp
	and rbp, -4
	lea rdx, [r11 + 4]
	add rdx, rbp
	mov rsi, -1
	cmovb rdx, rsi
	cmp rdx, rcx
	mov qword ptr [rsp + 40], rdi
	ja .LBB_50
	add rbp, 4
	add rdx, 3
	and rdx, -4
	mov qword ptr [rax], rdx
.LBB_51:
	mov rdi, rbp
	mov rsi, r12
	mov rdx, r14
	mov r14d, r10d
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rdi, qword ptr [rsp + 40]
	mov r10d, r14d
.LBB_52:
	movabs r9, 2305843009213693951
	mov rsi, qword ptr [rsp + 16]
	mov r8, qword ptr [rsp + 32]
	jmp .LBB_59
.LBB_60:
	mov esi, 4
	mov qword ptr [rsp + 40], rdi
	mov rdi, qword ptr [rsp + 8]
	mov qword ptr [rsp + 48], r11
	mov rdx, r11
	mov ebp, r10d
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov rdi, qword ptr [rsp + 40]
	mov r10d, ebp
	movabs r9, 2305843009213693951
	mov rsi, qword ptr [rsp + 16]
	mov r8, qword ptr [rsp + 32]
	mov rbp, rax
	test rbp, rbp
	jne .LBB_59
	jmp .LBB_62
.LBB_41:
	mov esi, 4
	mov rdi, qword ptr [rsp + 8]
	mov ebp, r10d
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov r10d, ebp
	movabs r9, 2305843009213693951
	mov rsi, qword ptr [rsp + 16]
	mov r8, qword ptr [rsp + 32]
	mov r12, rax
	test rax, rax
	jne .LBB_42
	jmp .LBB_33
.LBB_50:
	mov esi, 4
	mov rdi, qword ptr [rsp + 8]
	mov qword ptr [rsp + 48], r11
	mov rdx, r11
	mov ebp, r10d
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov r10d, ebp
	mov rbp, rax
	test rax, rax
	jne .LBB_51
	jmp .LBB_62
.LBB_55:
	mov esi, 4
	mov qword ptr [rsp + 40], rdi
	mov rdi, qword ptr [rsp + 8]
	mov rdx, r11
	mov dword ptr [rsp + 28], r10d
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	test rax, rax
	je .LBB_62
	mov rbp, rax
	mov rdi, rax
	mov rsi, r12
	mov rdx, r14
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rdi, qword ptr [rsp + 40]
	mov r10d, dword ptr [rsp + 28]
	jmp .LBB_52
.LBB_65:
	mov esi, 4
	mov rdi, qword ptr [rsp + 8]
	mov r12d, r10d
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov r10d, r12d
	mov r12, rax
	test rax, rax
	jne .LBB_30
	jmp .LBB_66
.LBB_34:
	mov esi, 4
	mov rdi, qword ptr [rsp + 8]
	mov dword ptr [rsp + 28], r10d
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	test rax, rax
	je .LBB_66
	mov r12, rax
	mov rdi, rax
	mov rsi, rbp
	mov rdx, r13
	call qword ptr [rip + memcpy@GOTPCREL]
	mov r10d, dword ptr [rsp + 28]
	jmp .LBB_31
.LBB_12:
	cmp r13, rdi
	jbe .LBB_19
	lea rdx, [4*r13]
	lea rbx, [4*rdi]
	test bpl, 3
	jne .LBB_14
	add rdx, rbp
	mov rax, qword ptr [rsp + 8]
	mov rax, qword ptr [rax]
	cmp rdx, qword ptr [rax]
	jne .LBB_19
	add rbx, rbp
	mov qword ptr [rax], rbx
.LBB_19:
	mov rax, rbp
.LBB_2:
	mov rdx, rdi
	add rsp, 56
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB_66:
	add r13, rbp
	mov rax, qword ptr [rsp + 8]
	mov rax, qword ptr [rax]
	cmp r13, qword ptr [rax]
	jne .LBB_4
	mov qword ptr [rax], rbp
	xor eax, eax
	jmp .LBB_2
.LBB_33:
	xor eax, eax
	jmp .LBB_2
.LBB_7:
	mov esi, 4
	mov rbx, rdi
	mov r14, r8
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov rsi, qword ptr [rsp + 16]
	mov r8, r14
	mov rdi, rbx
	mov rbp, rax
	test rax, rax
	jne .LBB_8
	jmp .LBB_4
.LBB_14:
	mov ecx, 4
	mov r14, rdi
	mov rdi, qword ptr [rsp + 8]
	mov rsi, rbp
	mov r8, rbx
	call bump_scope::allocator::shrink::shrink_unfit
	mov rdi, r14
	test rax, rax
	jne .LBB_2
	mov edi, 4
	mov rsi, rbx
	call qword ptr [rip + alloc::alloc::handle_alloc_error@GOTPCREL]
	jmp .LBB_16
.LBB_64:
	call qword ptr [rip + allocator_api2::stable::raw_vec::capacity_overflow@GOTPCREL]
	jmp .LBB_16
.LBB_62:
	mov edi, 4
	mov rsi, qword ptr [rsp + 48]
	call qword ptr [rip + alloc::alloc::handle_alloc_error@GOTPCREL]
.LBB_16:
	ud2
	mov r12, rbp
	mov rbx, r13
	jmp .LBB_69
	test rbx, rbx
	je .LBB_71
.LBB_69:
	lea rdx, [r12 + 4*rbx]
	mov rcx, qword ptr [rsp + 8]
	mov rcx, qword ptr [rcx]
	cmp rdx, qword ptr [rcx]
	jne .LBB_71
	mov qword ptr [rcx], r12
.LBB_71:
	mov rdi, rax
	call _Unwind_Resume@PLT