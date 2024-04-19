inspect_asm::alloc_iter_u32::try_mut_up:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	mov eax, 4
	test rdx, rdx
	je .LBB_10
	mov rcx, rdx
	shr rcx, 61
	jne .LBB_16
	shl rdx, 2
	mov r8, qword ptr [rdi]
	mov rcx, qword ptr [r8]
	mov r8, qword ptr [r8 + 8]
	add rcx, 3
	and rcx, -4
	mov r9, r8
	sub r9, rcx
	cmp r9, rdx
	mov rbx, rdx
	jb .LBB_14
	test rcx, rcx
	je .LBB_16
	and r8, -4
.LBB_5:
	sub r8, rcx
	shr r8, 2
	mov qword ptr [rsp], rcx
	mov qword ptr [rsp + 8], 0
	mov qword ptr [rsp + 16], r8
	mov qword ptr [rsp + 24], rdi
	xor r15d, r15d
	mov r14, rsp
	xor edi, edi
	jmp .LBB_7
.LBB_6:
	mov dword ptr [rcx + 4*rdi], ebp
	inc rdi
	mov qword ptr [rsp + 8], rdi
	add r15, 4
	cmp rdx, r15
	je .LBB_11
.LBB_7:
	mov ebp, dword ptr [rsi + r15]
	cmp qword ptr [rsp + 16], rdi
	jne .LBB_6
	mov rdi, r14
	mov r12, rsi
	call bump_scope::mut_bump_vec::MutBumpVec<T,A,_,_,_>::generic_grow_cold
	mov ecx, eax
	mov eax, 4
	test cl, cl
	jne .LBB_16
	mov rsi, r12
	mov rdx, rbx
	mov rcx, qword ptr [rsp]
	mov rdi, qword ptr [rsp + 8]
	jmp .LBB_6
.LBB_11:
	cmp qword ptr [rsp + 16], 0
	je .LBB_10
	mov rax, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 24]
	lea rdx, [rax + 4*rdi]
	mov rcx, qword ptr [rcx]
	mov qword ptr [rcx], rdx
	jmp .LBB_17
.LBB_10:
	xor edi, edi
	jmp .LBB_17
.LBB_14:
	mov r15, rsi
	mov esi, 4
	mov r14, rdi
	mov rdx, rbx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_greedy_in_another_chunk
	mov rcx, rax
	mov eax, 4
	test rcx, rcx
	je .LBB_16
	mov rdi, r14
	mov rsi, r15
	mov r8, rdx
	mov rdx, rbx
	jmp .LBB_5
.LBB_16:
	xor eax, eax
.LBB_17:
	mov rdx, rdi
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret