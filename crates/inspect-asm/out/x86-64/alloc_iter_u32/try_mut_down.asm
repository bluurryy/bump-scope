inspect_asm::alloc_iter_u32::try_mut_down:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	test rdx, rdx
	je .LBB_12
	mov rax, rdx
	shr rax, 61
	jne .LBB_15
	shl rdx, 2
	mov rax, qword ptr [rdi]
	mov rcx, qword ptr [rax]
	mov rax, qword ptr [rax + 8]
	and rcx, -4
	mov r8, rcx
	sub r8, rax
	cmp r8, rdx
	mov rbx, rdx
	jb .LBB_3
	add rax, 3
	and rax, -4
	je .LBB_15
.LBB_6:
	sub rcx, rax
	shr rcx, 2
	mov qword ptr [rsp], rax
	mov qword ptr [rsp + 8], 0
	mov qword ptr [rsp + 16], rcx
	mov qword ptr [rsp + 24], rdi
	xor r12d, r12d
	mov r15, rsp
	xor r14d, r14d
	jmp .LBB_7
.LBB_10:
	mov dword ptr [rax + 4*rcx], ebp
	lea r14, [rcx + 1]
	mov qword ptr [rsp + 8], r14
	add r12, 4
	cmp rdx, r12
	je .LBB_11
.LBB_7:
	mov ebp, dword ptr [rsi + r12]
	mov rcx, r14
	cmp qword ptr [rsp + 16], r14
	jne .LBB_10
	mov rdi, r15
	mov r14, rsi
	call bump_scope::bump_vec::BumpVec<T,_,_,A>::generic_grow_cold
	test al, al
	jne .LBB_15
	mov rsi, r14
	mov rdx, rbx
	mov rax, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 8]
	jmp .LBB_10
.LBB_11:
	mov rax, qword ptr [rsp + 16]
	test rax, rax
	je .LBB_12
	mov rsi, qword ptr [rsp]
	mov rbx, qword ptr [rsp + 24]
	lea rdx, [rsi + 4*r14]
	lea rax, [rsi + 4*rax]
	not rcx
	lea rdi, [rax + 4*rcx]
	cmp rdi, rdx
	jae .LBB_16
	mov rax, rsi
	jmp .LBB_17
.LBB_12:
	mov eax, 4
	xor r14d, r14d
	jmp .LBB_18
.LBB_16:
	lea rdx, [4*r14]
	mov r15, rdi
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, r15
.LBB_17:
	mov rcx, qword ptr [rbx]
	mov qword ptr [rcx], rax
	jmp .LBB_18
.LBB_3:
	mov r15, rsi
	mov esi, 4
	mov r14, rdi
	mov rdx, rbx
	call bump_scope::bump_scope::BumpScope<_,_,A>::alloc_greedy_in_another_chunk
	test rax, rax
	je .LBB_15
	mov rdi, r14
	mov rsi, r15
	mov rcx, rdx
	mov rdx, rbx
	jmp .LBB_6
.LBB_15:
	xor eax, eax
.LBB_18:
	mov rdx, r14
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret